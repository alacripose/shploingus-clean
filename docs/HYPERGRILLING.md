# HYPERGRILLING — the question ledger, append-only

> Every **material** question put to the user, with the answer in the user's own
> words, what the answer changes, and where its reference traces were found.
> A later answer **supersedes** an earlier one; both stay readable. This is the
> hypergrilling protocol's ledger. `LEDGER.md` carries the decisions that follow
> (`C-48` onward), and `TODO.md` carries the plan of record.
>
> **High agency mode is declared explicitly for the round-7 invocation
> (2026-09-29)** — per-invocation, non-inheriting, logged here and in `C-48`.
>
> The old trees (`~/Documents/shploingus`, `shploingus-game/references`) are
> **reference only, never authority** — the user's ruling, 2026-09-29. Where an
> answer below cites a reference, it is cited as *evidence of a shape*, not as a
> decision this repository has admitted.

---

## Round 7 — the debug views, 2026-09-29

Asked after the audit (`LEDGER.md` **C-48**) of the clean tree's own
documentation — `LEDGER.md`, `STATUS.md`, `TODO.md`, `SHAPERS.md`,
`DESIGN-SPEC.md`, `MODS-MAP.md`, `APPEARANCE.md` — and of the reference material:
`docs/DEBUG-VIEWS.md`, `HANDOFF-NEXT-AGENT.md`,
`crates/diffusion-core/src/cuboid.rs`, `crates/voxel-client/src/debug.rs` and
`render/shell.rs` (the dirty client), the FC-6.7 and SPEC-6 rows, the design
package's 20 Hz record, `portal_simulator.py`, and
`shploingus-game/docs/plan/00-authority.md`.

### H-7.1 — The in-client world-shaper view: what must it show?

**The question.** Four candidate contents were offered: terrain stages live,
per-voxel attribution, voxel shaper geometry, and stage toggles.

**Answer (user's words).**

> There is a really nice cuboid overlay that appears that shows elements of the
> world in the dirty branch already - so like world root. The idea is to expand
> that feature into a vector debug view that will outline areas that change
> within a chunk, representing the actual change that happened, including when
> the changes to the chunk are saved to the disk

**What it changes.** The view is not a per-stage readout. It is a **vector
(outline) view of change**: areas **within a chunk** that changed are outlined,
each outline representing the actual change that happened — and the view also
marks **when a chunk's changes are saved to disk**. So the view's subject is the
world's *modification record* and its *persistence state*, drawn as vectors over
the world. The reference trace: the dirty client already draws world-space boxes
as projected 3D outlines with labels at their own position
(`render/shell.rs`: *"World lanes are drawn as outlines in the world. Their
bounds are in world…"*), and `docs/DEBUG-VIEWS.md` scopes every view by a
**cuboid selection** whose `cells()` **refuses** rather than clamps.

**Owed by this answer (named, not hidden).** The clean tree has **no file I/O at
all** — `std::fs`/`File::create`/`read_to_string` over `crates/` returns zero
matches — so "saved to the disk" is machinery that does not exist yet, not a view
over something that does. Follow-up question **H-7.14** asks its scope.

### H-7.2 — What makes "in real time" true?

**The question.** How the client's tick advances, given the world is a pure
function of `(seed, tick, declared period)` and the shipped client holds
`tick = 0`.

**Answer (user's words).**

> In the city builder prototype there was a deffered 20hz tick system which also
> made it to ala-cities-target.exe for enhanced performance since ticking the
> world would not immediatley tick everything at once while allowing global time

**What it changes.** The client grows a **fixed 20 Hz world clock** with
**global time**, and work is **deferred across ticks** — one base tick, systems
on their own declared cadences, so a tick does not run everything at once.
Reference trace (evidence of shape): `ala-cities` exports
`sim::{Clock, World, SIM_HZ}`; its frame loop uses `sim_dt = 1.0 / SIM_HZ`;
governance runs on `tick % 4 / % 20 / % 40 / % 200 / % 600` cadences; its
streamer is tick-budgeted (`max_builds_per_tick`) in `asset_streaming.rs`; and it
autosaves on a `TICKS_PER_DAY × DAYS_PER_MONTH` boundary — the same boundary a
save marker in H-7.1's view would show.

### H-7.3 — How does a cuboid get made in the client?

**The question.** The reference left three candidates (two corners, drag and
extrude, type min/max).

**Answer (user's words).**

> Depends because there are different levels. One for the player when they are
> selecting blocks to interact with, one for viewing debug lanes, and the actual
> shapers themselves that modify the world

**What it changes.** The cuboid instrument is **one instrument with three
uses**, not one UI:
1. **player interaction** — selecting blocks to interact with (the dig/build
   path the bin already has);
2. **debug-lane viewing** — the scope a view reads and draws;
3. **the shapers that modify the world** — a selection the shaper tools apply
   to (FC-6.7's *"world shaper wand"*, FC-6.1's *"14 parametric build
   primitives"*).
The reference's §4 left the *making* open; the clean tree must therefore decide
it per use rather than once. Follow-up question **H-7.16** asks what level 3 is
this slice.

### H-7.4 — Where do the debug views live?

**The question.** World overlay plus a HUD panel, the reference's tab shell, or
a side pane — with SPEC-6's recorded defect quoted: *"shaper UI hidden when esc
menu hidden or f1 toggled"*.

**Answer (user's words).**

> The previous shell implementation was missing the true window management
> powers of the portal_simulator.py reference document (must use rust shell
> implementation as project specifies)

**What it changes.** The client grows a **Rust shell** whose windows have the
powers of `shploingus-game/references/portal_simulator.py` — a 4,252-line
PySide6 reference whose machinery is: `WindowState {NORMAL, MINIMIZED, MAXIMIZED,
CLOSED}`, `TitleBarPosition {TOP, BOTTOM, NONE}`, `WindowLayoutPolicy`
(title height, border width, resize margin, min width/height, title position,
show min/max/close/icon), `WindowBehaviorPolicy` (allow drag, resize, maximize,
minimize, close), `WindowGeometry` (with `previous` for restore), title-bar and
pane elements, `PortalWindow`, and a `Portal` desktop with a taskbar that
minimizes/restores windows. It is **not** a Python dependency: the clean tree is
Rust, and the reference supplies the **powers** the previous shell lacked.
Follow-up question **H-7.15** asks the first cut.

### H-7.5 — Should the debug views be declared data or hand-written code?

**The question.** Given the user's own statement that the recipes/packages
system exists *"so we can lessen the amount of work any agent or person has to
do"*.

**Answer (selected).** **All declared.**

**What it changes.** A **view is a data record** — what it selects, which
stages/shasers it reads, its labels and its lane class — and **one generic
renderer draws any declared view**. Adding a view is data, never Rust. Where the
record is admitted is follow-up question **H-7.17**; the precedent is
`shploingus-game`'s recipe → package route (`.shspec` → `.shpkg`, signed,
refuse-on-mismatch) and this tree's own `voxel-package` intake (manifest,
digest, refusal).

### H-7.6 — May the debug surface step the world it measures?

**The question.** Seed 2468 is the shipped world; is a seed control wanted?

**Answer (user's words).** *"seed is set per world"*

**What it changes.** Worlds are **seeded**, and the seed is a declared property
of a world rather than a global constant — so a debug surface may name a
different world without touching the shipped one, and `GAME_SEED = 2468` becomes
the shipped **world's** seed, not the game's only possible one. Tick stepping and
advance are H-7.2's clock.

### H-7.7 — Lane colours: port the reference's six hues exactly?

**Answer (user's words).** *"port according to the new requirments"*

**What it changes.** The lane system is **ported** into the new structure (six
classes, the frozen vocabulary and its digest, labels at the lane's own
position) and its colours are to be adapted **to the new requirements** — the
declared-view renderer and the clean tree's design system — rather than lifted
verbatim. The exact hues and the digest pin are therefore **not** copied as
authority; the *shape* is. Follow-up question **H-7.15** carries the shell cut;
the colour derivation lands with the renderer and is recorded when it does.

### H-7.8 — What is the completion bar for this branch?

**Answer (selected).** **Playable world · Debug views · Packages & mod UI.**
Not selected: the bonus goals (gauges, Lego-Movie water, progression,
contraptions).

**What it changes.** "The completed program" for this branch is the playable
world, the debug views visible in the client, and the package/mod API with its
decorated in-game UI — the four bonus goals stay ranked below those three.

---

## Round 7, second pass — from the first answers, 2026-09-29

Asked after the first pass, and after the traces it opened were read
(`ala-cities-design-package/src/persistence/{mod,lineage}.rs` and
`src/sim/mod.rs` for the save question; `portal_simulator.py` for the shell).

### H-7.9 — What grounds "the actual change" the vector view outlines?

**Answer (user's words).**

> Just a simple outline over the changed area for a small amount of time - the
> previous 3d cuboid debug view in the dirty branch worked but only when the
> shell was open, so the vector view should be able to do that but with the
> terrain shaper debug views

**What it changes.** The vector view is **transient**: an outline over the
changed area, shown for a *small amount of time*, then gone. It must work
**with the shell closed** — the dirty branch's 3D cuboid debug view only drew
while the shell was open, and that dependence is the defect this replaces. And
it carries the **terrain shaper debug views**: the shaper observability rides
this view rather than a separate readout.

### H-7.10 — What is the save the view marks?

**Answer (user's words).** *"Requires more information because we have a way of
saving storage with the highly expensive chunk format"*

**Status: needs information, then a ruling.** The traces were read and are
summarised here so the next question carries the information it asks for:

* **The prototype's two layers.** `ala-cities`’ city save is a single RON
  document (`sim::World::save`) written through a **temporary file and an
  atomic rename**, stamped with a `format_version`, and it **refuses to save**
  if the in-memory stamp differs from what the build writes — the comment's own
  reason is *“a 65k-tile world in JSON is a text file nobody wants to open”*.
  Beside it sits the **world lineage envelope** (`WORLD_LINEAGE_SCHEMA =
  "ala-cities/world-lineage/v1"`, `WorldLineageEnvelope`, `SaveReceipt` naming
  the digests written, digest-mismatch refusal on load, exact-rational
  baselines, a `ProjectionCheckpoint`, an input **journal** with outcomes, and
  `replay` reporting the **first difference** against a named projection;
  a legacy save is **refused by version**, not migrated silently).
* **This tree's numbers.** A chunk with terrain in it costs **90,481 B** as an
  octree and **1,327 B** as a mesh (C-29), and the world is a pure function of
  `(seed, tick, the modification record)`. So the “highly expensive chunk
  format” is the octree-per-chunk save, and the cheap alternative is the
  **record plus regeneration** — exact here *because* determinism is a contract.

Round-3 question **H-7.16** puts the choice with those numbers attached.

### H-7.11 — Which cut is the Rust shell's first deliverable?

**Answer (selected).** **Core windows** — movable, resizable from edges, a
minimum size, focus/z-order, close. Window content comes from a declared record.

### H-7.12 — Where does a declared view record live?

**Answer (user's words).**

> We ar basically making the v3 version that combines all of the past projects
> working code, using the complete recipe + resume + civilian + governance +
> security model for enabling hyperspeed development which will allow the game
> to be developed properly

**What it changes.** The program is **v3**: the combination of all past
projects' working code under the complete **recipe + résumé + civilian +
governance + security** model, for hyperspeed development. The declared-view
question is therefore answered at the program level: **declarations ride the v3
model**, and the concrete admission route (this tree's intake vs the
`shploingus-game` core) is a plan question, not a view question. Traces:
`shploingus-game` (`recipes/*.shspec`, `packages/*.shpkg`, `docs/plan/` — the
résumé integration L8–L9, governance and citizens in L2, the security scope D6).

### H-7.13 — What does cuboid level 3 ("the shapers that modify the world") mean?

**Answer (user's words).**

> generation with debug vector view, every test ran in game using the debug
> lane, every cargo test actually visible, so missing features are known by the
> client itself

**What it changes.** Two things, and the second is the larger:

1. level 3 this slice is **generation with the debug vector view** — the shaped
   world and the view over it, not a separate authoring tool yet;
2. **every test runs in-game through the debug lane, and every `cargo test` is
   actually visible** — so a missing feature is known *by the client itself*.
   The client is no longer merely where a feature is looked at (C-40); it is
   where the suite's own state is readable. Round-3 question **H-7.17** asks the
   mechanism.

### H-7.14 — Confirm the 20 Hz clock shape?

**Answer (user's words).** *"Optimized for the new requirements"*

**What it changes.** The clock is **designed for the new requirements**, with
the 20 Hz prototype as evidence of **shape** (one global time; deferred
cadences so a tick never runs everything at once; render at display rate) and
not as the frozen number. The requirements to optimise against are this tree's
own: high-refresh render, determinism as a contract, and the F9 stage graph's
“each process declares its own clock”.

### H-7.15 — Which window does the shell open first?

**Answer (user's words).**

> Shell is the upper tab bar that was missing key design elements that
> shploingus-clean uses

**What it changes.** The shell **is the upper tab bar** — the tab strip the old
client had, rebuilt with the key design elements **this** tree uses
(`voxel-ui`: the MD1/HIG rules, the four-bucket constants, the ink ladder, the
contrast floor, the 48 dp target floor, reduced motion). Windows open **from**
the bar; the bar is not optional chrome. Round-3 question **H-7.18** asks which
tabs it carries.

---

## Round 7, third pass — from the second answers, 2026-09-29

### H-7.16 — What should the save store?

**Answer (user's words).** *"Ask based off of the current system used in
shploingus-clean"*

**What it changes.** The save is designed **from this tree's own machinery**, not
imported from the prototype: the append-only **earthworks record**
(`voxel-shapers::earthworks`, *an unrecorded modification is invisible*), the
**seed and tick** the world is a function of, and the digest machinery
(`voxel-model::identity`). Chunks are recomputable, so the record plus the world
inputs are the save; the prototype's envelope/journal/replay ideas inform the
shape (digests, receipt, first-difference replay) without being the authority
for it. Confirmatory question **H-7.20** states the resulting shape back.

### H-7.17 — How does “every cargo test actually visible” work?

**Answer (user's words).**

> Special debug shell console window that comes from desktop_simulator.py rust
> deriviation for the shell view, plus a log window

**What it changes.** The shell carries a **console window** — a Rust derivation
of the simulator reference's console (`ConsoleWidget`: command input, execute,
output, `append_system_message`, `render_logs`, an event log fed by
`on_event_logged`) **plus a log window**. The console is where a test run becomes
visible in the client and where commands are given; the log window is the event
record. The reference file the user names is `portal_simulator.py` in
`shploingus-game/references/` (4,252 lines, PySide6); its console and its
window/portal machinery are the derivation source.

### H-7.18 — Which tabs does the upper bar carry?

**Answer (user's words).**

> Optimized for the new requirements, missing different tabs that were proposed
> but not added, detailed ui spec and ledgers in the dirty branch and the
> ala-cities will require more questions

**What it changes.** The tab set is **optimised for the new requirements** and
the source for it is the **detailed UI spec and ledgers in the dirty branch and
`ala-cities`**, which must be read before the question can be put properly. The
dirty branch's own tab list (`crates/voxel-client/src/shell/tabs.rs`) and
`ala-cities`' `UNIFIED_DESIGN.md` / `ICON_STANDARD.md` are the spec sources;
tabs that were *proposed but never added* are explicitly in scope. Round-4
questions follow that reading.

### H-7.19 — Where do the v3 core and this game tree meet?

**Answer (selected).** **Join at the Mods window** — keep building separably;
the first contract between the trees is the Mods window's declared view of
packages and their APIs. This keeps C-1's boundary honest: the game consumes the
package API, and the first visible consumer of it is the Mods window.

---

## Round 7, fourth pass — from the third answers, 2026-09-29

### H-7.20 — The save shape, stated back from this tree's own machinery

**Answer (user's words).**

> Yes but to avoid restreaming the clients chunks every ticks there must be a
> client side chunk cache

**What it changes.** The save is the **record + seed + tick**, and the client
keeps a **chunk cache** so a tick does not re-stream the world it already holds.
The cache is a client-side working set, not the durable artifact — the
prototype's own envelope discipline applies as *shape*: digests, a receipt,
digest-mismatch refusal on load, atomic write (temp file + rename), a legacy
save refused by version. The distinction the cache introduces is now explicit in
the design: **durable truth (the record) vs resident working set (the cache) vs
regenerable function (the world)**.

### H-7.21 — The tab set

**Answer (selected).** **Propose from the specs** — the plan proposes the set
from the dirty branch's UI spec, this tree's ledger, and `ala-cities`' design
docs, then it comes back as a question. The traces read for it:

* **The dirty branch's six, named by the user**: `inventory/crafting` (dual
  pane), `all systems crafting trees` (dual), `tickets and governance observer`
  (dual), `mods`, `docs`, `settings` (ledger events 0004/0010/0017).
* **What happened to them since**: `docs` was removed as a window and its
  *generated-from-the-loaded-surface* principle moved into the **Mods** tab
  (this tree's `shell/tabs.rs`); a **Tests tab was ruled out** — *"There is no
  Tests tab and no separate test surface. A test is visible by looking at the
  thing it tests"* (event-0013) — and the lanes on every window are the test
  surface.
* **The spec sources the user names**: `docs/MD1.md` (the MD1 map: the
  `native-immersive` profile, the shell as a **solid** layer that must not pass
  input through, every window keyboard-reachable, the four-bucket provenance
  rule, the token values) and `ala-cities`' `UNIFIED_DESIGN.md` (six design
  lineages; the Popit **contextual tool surface** and the Advanced-Warfare
  **legibility-under-load** surface as the two targets).

### H-7.22 — The console + log window's first cut

**Answer (user's words).**

> Depends because I did spec out a feature for the windows to sit in tiles in
> the shell's actual surface over the game

**What it changes.** A **tiled shell surface** is already specced by the user
and the console/log cut depends on it. The trace found: event-0004 — *"Windows
are movable and tile cleanly into the transparent background with readable
text"*, the strip dropping from the top edge over a **blurred, still-running**
game; event-0018 — *"the player drags window titles into slots and the windows
snap"*, arrangement policy as a package. Where the fuller tiling spec lives is
**H-7.24**, asked rather than guessed.

### H-7.23 — Does the arrangement-as-a-package ruling stand?

**Answer (selected).** **Decide with the Mods window** — the arrangement package
is the first package the Mods window shows, so the join point and the ruling
land together. Traces: event-0018 (arrangement is a real loadable package; the
loader is built first; the shell holds only transient state) and H-7.19 (the v3
join is the Mods window).

---

## Round 7, fifth pass — from the fourth answers, 2026-09-29

### H-7.24 — Where is the tiled-shell-surface spec?

**Answer (user's words).**

> those two events while still allowing the free floating windows. a feature
> should be allowing the user to pin shell simulator window to the game ui so
> they can choose how their heads up display looks

**What it changes.** The tiling spec **is** event-0004 + event-0018 (*"tile
cleanly into the transparent background"*, *"drag titles into slots and the
windows snap"*), **plus free-floating windows**, **plus a new feature**: the
player can **pin a shell window into the game UI** and so choose how their HUD
looks. Pinning is the join between the shell and the HUD — a pinned window is
part of the head-up display, an unpinned one is a shell window.

### H-7.25 — Does the proposed tab set stand?

**Answer (selected).** **Yes — build that set.** The bar carries the six the user
named (inventory/crafting · working trees/crafting trees · tickets/governance ·
mods · docs · settings); the debug console and the log are windows opened from
the bar; the vector change view is world-space and has no tab; the tool surface
(the world shaper wand) opens on the thing under the cursor; there is no Tests
tab, because the lanes on every window are the test surface.

### H-7.26 — Which proposed-but-never-added tabs must the set restore?

**Answer (user's words).**

> tools, map/plan, popit is also used as a design element for the general surface
> design, including call of duty advanced warfare. Even the material design 1
> full spec as a skill's own animation properties and material properties.. a
> full integration with the systems allowing for nice design that goes through
> the world and the ledger, a single contiguous project

**What it changes.** **`tools`** and **`map/plan`** join the set. Popit and
Advanced Warfare are **design elements of the general surface**, not only of two
special surfaces; **the full MD1 spec applies**, including its animation
properties and its material properties; and the design must be a **full
integration** — going *through the world and the ledger* — in **a single
contiguous project**.

### H-7.27 — Which design doctrine is binding?

**Answer (user's words).**

> There are more design lineages missing like those from previous projects, The
> design lineage has been the hardest to carry through

**What it changes.** The lineage set is **not closed** — more lineages from
previous projects are missing — and carrying them through is named as the
project's hardest problem. The plan must therefore (a) collect the lineages from
the previous projects, (b) declare one contiguous system, and (c) carry it
through the world, the shell and the ledger rather than skinning the client.
Round-6 questions ask the scope of (a).

### H-7.28 — What form does the chunk cache take?

**Answer (user's words).**

> depends because the implementation will require loading worlds so terrain
> shapers will also need some efficient way of having their information logged
> for replays and saves so people don't start with a fresh world every time

**What it changes.** Worlds **load**; the terrain shapers need an **efficient
log of their own information** so that a world can be **replayed and saved** and
a player does not start fresh each time. That is the F9 shape (pure
`(seed, tick, declared period)`, lazily advanced) plus a **shaper log** — the
per-stage facts a replay and a save need — and it is the reason the chunk cache
question is not answerable yet: the cache is downstream of what the log holds.

---

## Round 7, sixth pass — asked from the fifth answers

| # | The question | Answer |
|---|---|---|
| H-7.29 | Which previous projects' design lineages join the system | open |
| H-7.30 | What the shaper log records, and where it is kept | open |
| H-7.31 | The first build slice, confirmed against the plan's order | open |

---

## Round 7, seventh pass — the lineage research, 2026-09-29

The instruction for this pass:

> you will have to do hyper diligent research to understand the previous lineage's
> different implementations of the ticket ledger system, all of them missing key
> elements others have

and the locations: `desktop_simulator.py` (its **primitive version** — the file on
disk is `F:\Documents\simulators\desktop_portal.py`, the parent of
`portal_simulator.py`), the extra projects on `F:`, and the two Documents folders
(`C:\Users\ala\Documents`, `F:\Documents`, `D:\Users\alacrity\Documents`).
Paths below are **evidence of where a thing lives**, never authority over what
this tree must do (C-48).

### Scope note — what the L-series is, and is not (C-48)

**Everything in `L-1` … `L-6` is reference material read from other trees. None of it rules
this tree.** `docs/LEDGER.md` C-48 says so, and the user has said so twice: *"The old
documentation outside of shploingus clean is only reference, not authority."* The L-series
exists to answer **why** another project made a choice, so this tree can decide whether that
shape fits *its* format — not to import a decision tree.

The only authority in this repository is: `docs/LEDGER.md`, `docs/STATUS.md`,
`docs/SHAPERS.md`, `docs/DESIGN-SPEC.md`, `docs/MODS-MAP.md`, `docs/APPEARANCE.md`,
`docs/TODO.md`, this file's **user answers**, and the user's direct rulings.

One distinction inside the L-series, because it is easy to get wrong in both directions:

* The **three design lineages the user selected by name** in round 6 — `material-design-1`,
  `fma-bars`, `impeccable` — are adopted **by that selection**, so their specs bind here.
* Every other tree named below (`shploingus` dirty, `shploingus-game`, `ala-cities`,
  `income-engine`, `ala-system`, `simulators`, `Bonsai-demo`, the skill library) is
  **reference**: read for rationale, never cited as a ruling.

**A recorded failure, because it happened in this session.** While writing L-1…L-6 the author
of this file filed other trees' decisions under the words *"Ruled"*, *"binding, carried into
this tree"* and *"settled by ruling, not open"* — the precise failure the user had already
named (*"you are starting to organize projects entire decision trees as authority instead of
understanding why those choices were made"*). The headings below are corrected to
reference-language; the facts stay, the authority claim does not.

### L-1 — The implementations as found (reference)

**1. `C:\Users\ala\Documents\shploingus` (dirty branch — reference only).**
`docs/DEBUG-VIEWS.md`, `crates/voxel-client/src/debug.rs` (1382 lines: the six
`LaneClass` hues, the frozen 14-row `LANE_VOCABULARY`, `vocabulary_digest()` =
17_908_169_584_511_756_368, `DebugOverlay`), `crates/voxel-client/src/shell/tabs.rs`,
`ledger/shploingus-slice1.jsonl` (76 events: `previous_event_hash`,
`integrity_status: append_only`, `supersedes`, actor/session/schema_version),
`crates/diffusion-core` (`hypergrilling.rs`, `changelog.rs`),
`crates/voxel-package`, and the two disposition files (SPEC-6
`cuboid_test_shaper_exists_in_current_build`; FC-6.7 *world shaper wand*).
**Owns:** the lane/debug-view vocabulary and the hash-linked event envelope.
**Lacks:** any read-back validation (no ticket is ever checked against the world),
no seasons, no refusals collected, no retirement taxonomy in code.

**2. `F:\Documents\income-engine`** (`income_core.py`, `cua_worker.py`, `seed.py`,
`dashboard.py`, `tests/`) + the `income-engine` skill. **Owns:** the money-truth
chokepoints — *a dollar is not income until a receipt cites it* (`ledger add`
without `--evidence` refused; evidence naming a local file that does not exist
refused; model output is never a receipt); *an agent cannot verify money and
cannot clear a gate* (`--source agent:<name>` refused at both chokepoints); the
per-channel autonomy ceiling with five hard-denied actions (`create_account`,
`enter_credentials`, `move_money`, `accept_terms`, `sign_contract`) that become a
**human gate, not a silent skip**; *effort is a budget, not a label* (under red
capacity only green work is due). One SQLite file is truth; CLI and GUI are views.
**Lacks:** no world, no shapers, no windows; its receipt rule has no analogue for
"what the world must show".

**3. `C:\Users\ala\AppData\Local\Temp\opencode\ala-cities-design-package`** (the
city builder's source-of-record — `~/ala-cities` now holds only `target/`).
`src/gov/mod.rs` (1823), `src/gov/season.rs` (416), `src/agentledger.rs` (272),
`src/agency/{action,actor,evidence,selection,task}.rs`, `src/observer/*`,
`src/persistence/{journal,lineage,replay}.rs`, `src/materials/ledger.rs`,
`src/matter/ledger.rs`. **Owns (read this pass):**

* *The governor decides before the world changes*, and a refusal names the
  operations that **would** have been accepted — a refusal you can act on.
* *A governance layer must not fail open*: an unreadable or unparsable governor
  file is `LoadState::FailedClosed` with **zero** permitted ops, never a quiet
  fall back to the permissive default. An unknown profile is refused, not guessed.
* `ALL_OPS` (8 names) is the whole operation vocabulary; a name outside it is
  refused at proposal time and the refusal quotes the list. `look`/`inspect` are
  in the vocabulary on purpose — a read-only profile must be able to describe itself.
* `Ticket { id, contract, kind: Build|Case|Intervention|Material, status:
  queued|open|active|blocked|review|complete|rejected|handled|dropped, objective,
  gate, opened_tick, closed_tick, terminal: Option<RetirementReason>, supersedes,
  governor_version, case_key, count, expectation, roads_before, evidence, actor,
  corrected }`.
* **`Expectation` is what the world must show, read back from the world, never
  inferred from the request that filed the ticket** — `RoadTiles`, `ZonedTiles`,
  `BuildingAt`, `Demolished`, `MaterialOf{tile,part,family,anchor,level}`. And
  `Expectation::tiles()` exists to answer a stale claim's real question: *did a
  later recorded action explain this change, or did the world move with nothing on
  the record to account for it?* Those are different findings and only one is a defect.
* `MaterialOf` is the **claim, not the condition**: what a reader compares is the
  as-built claim, while present condition is a number the world carries (aging is
  never read as a record contradicted by its own weathering).
* **Case sampling dedupes by case identity**: four hundred hungry citizens in one
  district are **one** case reading `count: 400`. The count is real, not decorative.
* `RetirementReason` — the 14-token taxonomy, with `is_validated()` true only for
  `CompletedAndValidated`.
* `Evidence { grade: "A" external oracle | "B" read back from world state | "C"
  self-report }` — the grade is on the record.
* `Correction` is append-only with `old_value`/`new_value`/`authorized_by`, and the
  ticket carries `corrected: Option<String>` — **never a mutation of `terminal`**.
* The board must **drain**: every ticket reaches a terminal state within bounded
  sim-seconds, and closes *unverified* when the world did not show the result,
  because *"cannot stall" must never be implemented as "auto-promote to success"*.
* `Season` on disk: `index.json`, `season.json`, `events.jsonl` (the one log),
  `tickets/ retirements/ evidence/ corrections/`.
* **Ids come from the directory, never from a counter** — a process killed mid-run
  leaves a counter behind its own directory, and trusting it hands out an id that
  already exists, which the append-only writer would then have to overwrite. The
  writer **refuses to overwrite an existing record** (`AlreadyExists`).
* **A corrupt record loses that record, not the file** — counted in
  `corrupt_records`, logged, never hidden; a bad event line is counted and the
  rest of the log still loads.

**Lacks:** no hash chain (events carry tick/kind/detail, no `previous_event_hash`),
no authority-snapshot digest, no resource classes or per-agent budgets, no
independent reviewer/cross-grill, no four-way outcome classification, no context
arithmetic, no regression surface (nothing keeps "previously working behavior"),
and its `contract` is a bare string rather than the seven fields.

**4. `C:\Users\ala\.agents\skills\orchestrating-ticket-agents`** (plus the temp
`orchestrating-ticket-agents-workspace` and
`orchestrating-ticket-agents-retirement-candidate`; read this pass: SKILL.md,
`references/{ledger-and-state,authority-and-conformance,recovery-and-resources}.md`,
`assets/{retirement-template,status-projection,contract-template,cross-grill,self-grill}.md`).
**Owns:**

* *The evidence record is truth; model output is a candidate.*
* `ledger.jsonl` is canonical; `STATUS.md` is a **derived projection** that cannot
  authorize or change state, and if it disagrees the ledger wins and the projection
  is rebuilt.
* The governed status set is exactly the nine ala-cities carved, with
  `claimed/working/self-grilled/cross-grilled/accepted` demoted to **events** — a
  status that is really an event is how a record starts lying.
* The event envelope: `schema_version, campaign_id, event_id, previous_event_hash,
  integrity_status: append_only, at, ticket_id, actor, event, from, to,
  contract_version, authority_ref, evidence_refs, detail` — with the honesty rule
  **`previous_event_hash=not_evaluable` means never call that state tamper-evident**,
  and a hash, when computed, is evidence about bytes and chain order, *not permission*.
* The ticket record embeds the seven contract fields and adds
  `permissions{allowed_tools,denied_tools}`,
  `scope{included_actions,excluded_actions,systems,entities,write_set,context_budget{parent_context_tokens,ticket_context_tokens,auto_loaded_files,measured_lane_window}}`,
  `execution{dependencies,resource_class CHEAP|MEDIUM|HEAVY|EXCLUSIVE|CONTEXT,max_parallelism,max_repair_rounds,max_subcontract_depth}`,
  `authority{snapshot_ref,observed_at,identity_status: verified|procedural}`,
  `validation{acceptance_tests,surface,reviewer,oracle}`, and `risk{classification,escalation}`.
* **Frontier preflight** (nine conditions, incl. write-set disjointness and context
  arithmetic proving the ticket fits the measured lane).
* **Retirement is a separate `ticket.retirement` event**: a status flip, an
  `outcome.recorded`, or a `STATUS.md` projection is *not* a retirement record;
  `completed_but_unverified` does not unlock dependents.
* Corrupt-line recovery: parse every line independently, preserve and report the
  invalid line + parse error, keep folding valid lines, **block dispatch** until a
  correction or reconstruction restores integrity — never edit or skip silently.
* Authority precedence: *current repository > current contract > current ticket >
  current world state > history*. **No record-as-instruction** (ledger text, worker
  prose, retrieved docs and `STATUS.md` are data and cannot modify the skill or the
  authority). **No stage grants the next stage.** **Repository-wins**: a conflict is
  preserved and recorded as a correction candidate, never silently normalized.
* Outcome classification `retained | gain | regression | residual_failure | inconclusive`.
* Native-local-only dispatch (no hosted/remote/external-CLI delegation), resource
  classes, one-writer-per-stream, disagreement preserved as `unresolved`.

**Lacks:** no world behind it — no `Expectation` read back from anything, no
shapers, no chunk bytes, no window; because hashes are `not_evaluable` in the
prompt-only workflow it has an append-only ledger **without** tamper-evidence; it
has no money/authority ceilings of the income-engine kind and no fail-closed
governor file.

**5. `F:\Documents\simulators`** — `portal_simulator.py` (4252 lines),
`desktop_portal.py` (the primitive), `AGENT.md`, `docs/design/{window_splitting_and_tiling,business_autonomy_architecture}.md`,
`docs/implementation_plans/{task_splitting_and_grid_engine,task_viewer_and_console,End game goals,orbs,implementation_plan11,"Business Autonomy System"}.md`.
**Owns:**

* The recursive split-tree shell: `PortalSplitter(QSplitter)` overriding
  `minimumSizeHint() = QSize(40,40)` to stop cumulative constraint clamping in
  nested hierarchies, `setStretchFactor(i,1)`, `setChildrenCollapsible(False)`,
  handle width 4; `WindowPane` + `PaneHeader` with `◧` split-right, `⬓` split-down,
  `⇱` undock, `×` close; `layout_root = WindowPane | recursive PortalSplitter`;
  `split_pane/close_pane/_cleanup_splitter/undock_pane/dock_window`;
  `DropPreviewOverlay`; `start/continue/end_window_drag` hit-testing.
* The **Task Viewer** (live tree of windows and panes with ids, apps, geometry and
  state NORMAL|MINIMIZED|MAXIMIZED|FOCUSED; focus/min/restore/max/split/undock/end
  task; filter; auto-refresh; stats) and the **Console**
  (`Portal.log_event(level, category, message)` → `log_history` + `logEvent` signal
  + stdout, colour-coded `[SYS] [EVENT] [INFO] [WARN] [ACTION]` with timestamps, and
  a command prompt: `help, launch, list, focus, close, min, max, split, theme,
  clear, echo, stats`).
* The **operator control center**: GREEN/YELLOW/RED operator modes; BLUE/ORANGE/RED
  **physical blockers** that halt one step, emit a delegation sheet, and let
  non-physical work continue; model routing L0 deterministic code / L1 local / L2
  cheap cloud / L3 frontier under one rule — *the cheapest model that can reliably
  complete the task*.
* **bizdb** (projects matrix, **master tasks ledger**, revenue + ROI, persistent
  memory of decisions & lessons) and an **agents** window (routing allocation,
  worker fleet status, token/cost dashboard).
* `End game goals.md`: cross-compiles to Rust, GPU-accelerated main window, fake
  desktop easter egg, live-USB, a kiosk-mode security layer that prevents focus
  stealing, "benevolence towards people and a ruthless bloodlust for money &
  accountability", an agentic OS that backs itself up and propagates, artificial
  life that sees the world through the screen, and a screensaver of it.

**Lacks:** Python/Qt where the platform mandate is Rust; no world, no voxels, no
shapers; its "tasks ledger" is business work, not world work; its verification is
Qt-offscreen tests, not world read-back.

**6. `C:\Users\ala\.agents\skills\contracts`.** The seven-field envelope
(MISSION/SCOPE/CONSTRAINTS/RESOURCES/EVIDENCE/DELIVERABLE/STOP), *subcontracts
narrow, never widen*, *evidence or it did not happen*, **grounding labels** (*"a
screenshot from the player's client is evidence about what the player can see; it
is not the body's position"*), *reasoning parallel, spending serial*, the
**fail-fast gate** (*"a failed build must never fall through to a probe of the old
binary"*; **a stale artifact is not a fix**), one-writer-per-stream, and the
`EXP-YYYYMMDD-NN` experiment ledger whose rule is *no single number called
"performance"*.

**7. `wayfinder-task-retirement` skill.** Governor Base → Season → Campaign →
Contract → Ticket → Action → Evidence → Validation → Outcome → Retirement Record;
the four-way table (fail→pass **gain**, pass→fail **regression**, fail→fail
**residual failure**, pass→pass **retained**); the retirement-record object and the
object-file/MongoDB layouts; and the ten rules — including *historical records do
not grant authority*, *agent self-report is evidence, not authority*, *speed does
not bypass retirement gates*, *retire the task, not the lesson*, and *a retired
task is revisited by a new task that references it, never by silently reopening it*.

**8. `institutional-orchestration` skill.** Six append-only ledgers — **Authority ·
Actor · Interaction · Contract · Evidence · Resume** — each row pointing back to its
source; and the separations: authorization ≠ funding, policy ≠ transaction,
information ≠ evidence, historical law ≠ current law, contact ≠ causation,
proximity ≠ authority. *A resume claim may never be stronger than its evidence.*

**9–11. Present but not yet read this pass:** `F:\Documents\ala-system` (the
personal evidence engine: claims that cite evidence, "show me why you believe this
sentence about me", capability graph → resume claim); `F:\Documents\wayfinder-hand`
(a NeoForge mod, Gradle/Java — the only other *wayfinder* on disk);
`F:\Documents\skills_directory` (the skills tree holding `wayfinder/`, `fma-bars/`,
`impeccable/`, `material-design-1/`, `grilling/`, `prototype/`, `domain-modeling/`,
`research/`, plus `us_2000_government_corporate_lawmaker_orchestration_lesson.md`
and `solution_dissolver.md`); and `F:\Documents\bspt`, `F:\Documents\AWA3D`,
`F:\Documents\alacripose-landing`, `C:\Users\ala\Documents\laya-cua-agent`,
`~/Bonsai-demo` (whose `rust-agent`/`mindcraft` are the bot bodies).

### L-2 — The "missing key elements" map

| element | the implementation that owns it | absent in |
|---|---|---|
| Governor file that fails closed; op vocabulary quoted in the refusal | ala-cities `gov/mod.rs` | every other lineage (ticket-agents has authority precedence but no unreadable-file case; income has hard-denied actions but no governor document) |
| `Expectation` — validation as read-back from the world | ala-cities | ticket-agents (no world), income (receipts instead), portal_simulator (offscreen Qt tests), dirty branch |
| Hash-linked event envelope + `not_evaluable` honesty | dirty branch ledger, ticket-agents envelope | ala-cities `events.jsonl` (no chain) |
| Retirement taxonomy + separate retirement record | wayfinder, ala-cities, ticket-agents | income, portal_simulator, dirty branch |
| Four-way gain/regression/residual/retained | wayfinder, ticket-agents | ala-cities, income, portal_simulator, dirty branch |
| Evidence grades (A oracle / B read-back / C self-report) | ala-cities | ticket-agents (has `oracle` field, no grade), income (receipt-only), portal_simulator |
| Append-only on disk with dir-derived ids and record-not-file corruption | ala-cities `season.rs` | ticket-agents (single jsonl), income (SQLite truth), portal_simulator |
| Canonical ledger + derived projection (`STATUS.md`) | ticket-agents | ala-cities (no projection file), dirty branch |
| Money/identity chokepoints and an autonomy ceiling per channel | income-engine | every other one |
| Seven-field contract envelope + resource classes + context arithmetic | contracts, ticket-agents | ala-cities (`contract` is a string), income, portal_simulator, dirty branch |
| Subcontract narrowing; never-widen; BLOCKED instead of improvising | contracts | ala-cities, income, portal_simulator |
| Case sampling that dedupes by identity with a real count | ala-cities `sample_cases` | every other one |
| Six ledgers including Actor and Resume | institutional-orchestration | all others (they carry actor as a field) |
| The lane/debug-view vocabulary and the frozen digest | dirty branch `debug.rs` | ala-cities, ticket-agents, income, portal_simulator |
| Recursive split-tree shell, task viewer, console with levels, operator modes, model routing | portal_simulator + `desktop_portal.py` | shploingus-clean (it has **no** shell, no tabs, no console, no task viewer) |
| Cuboid selection that refuses rather than clamps (`cells() -> Result`, `SelectionBudget`, `estimate_selection_bounded`) | ala-cities `agency/selection.rs` | shploingus-clean |
| The vector change view over a chunk; the 20 Hz deferred tick (not everything ticks at once, global time preserved) | dirty branch cuboid overlay + ala-cities `SIM_HZ` | shploingus-clean |

Round-8 questions are the decisions this map forces.

---

## Round 8 — the decisions the lineage map forces, 2026-09-29

### H-8.1 — One ledger, or two?

**Answer (selected).** **One append-only record, two views.** The same canonical
record carries world actions (dug here, shaped there) and development facts (this
check ran, this feature is missing). `docs/LEDGER.md` and the in-game ledger window
are **two projections of one record**, so a disagreement between them is impossible
by construction — which is what "a single contiguous project" has to mean if it is
to be more than a slogan.

### H-8.2 — The console's command surface

**Answer (user's words).**

> Yes, game commands but also the ui will need a place for the game's own windows
> that are relevant when the shell is hidden to come from, so without an editor
> the pinning is the only way to keep a window open, with no other windows types
> than the shell's appearing, so we don't even have the api for the api for the
> shell system yet

**What it changes — three things, and the third is a blocker.**

1. The console is the **game's own command surface** (world + record commands).
2. **There are no game-native windows.** A window the player needs while the shell
   is hidden must be a *shell* window that has been **pinned**; without an editor,
   pinning is the only way to keep a window open. So the HUD is composed of pinned
   shell windows — which is exactly why the old `debug.rs` carried
   `LaneUnit::{SquarePixels, CubicWorldUnits}`: a window's extent unit is a
   declared property, not a per-window hack.
3. **The shell system has no API yet — not even the API for the API.** This is the
   first thing to build, and it is upstream of the debug views, the console, the
   Mods window and the HUD. The deliverable order in `TODO.md` §7 is corrected
   accordingly.

### H-8.3 — The tick ladder

**Answer (selected).** **Base 20 Hz with declared stage cadences.** `WorldSpec`
declares a **period per stage** — player edits and dirty meshes at ×1, water and
settling at ×4, erosion and deposition at ×20, geology and climate at
`TICKS_PER_DAY` — and a stage's period is part of the **determinism contract**,
not a perf knob (F12). The world stays pure in `(tick, address)`; the deferred tick
is the *schedule*, never state.

### H-8.4 — How a world claim is validated

**Answer (user's words).**

> depnds on when we want to add a replay viewer

**What it changes.** The validation gate is **downstream of the replay viewer's
arrival**. Live read-back (ala-cities' `Expectation`) is available immediately;
replay-to-first-difference becomes a *gate* only once a viewer exists to show it.
So the replay viewer is not a debug nicety — it is the thing that promotes replay
from tool to gate, and its scheduling is a real decision (round 9).

### H-8.5 — Delivery order, and what proof means

**Answer (selected).** **Packages & Mods first — proof is that the join works.**
The package format and the Mods window are the contract between the v3 core and
this game, and they land before the world and the views, which are then built as
content loaded through it.

### H-8.6 — Operator autonomy in the game

**Answer (selected).** **Development-only for now; the game is single-operator.**
The human is always the operator in-game; the autonomy ceiling, the capacity modes
and the hard-denied actions govern delegated *development* work. The game's scope
stays the world, the shell and the record.

---

## Round 9 — from the round-8 answers

| # | The question | Answer |
|---|---|---|
| H-9.1 | The shell API's shape (declaration vs trait vs package manifest) | open — re-asked as H-9.7 |
| H-9.2 | Pinning semantics and the HUD's composition | open — re-asked as H-9.8 |
| H-9.3 | When the replay viewer arrives (and so when replay becomes a gate) | open |
| H-9.4 | What "the join works" proves concretely in the Mods window | open |
| H-9.5 | The one record's file shape (world events and dev facts in one file) | open |
| H-9.6 | The smallest end-to-end proof to build first | open |

**Round 9 was answered only at H-9.1:**

> You have not done the required research inside of portal/desktop simulator

Correct — the research pass had read the simulators' *docs* and not the 4252-line
implementation. `F:\Documents\simulators\portal_simulator.py` and `desktop_portal.py`
are **byte-identical** (md5 `d7fb4c45a9ba2d0b81185c0d8842b801`), so the "primitive"
version the user remembers is an earlier iteration with no copy on disk, not a
second file. The remaining round-9 questions were left blank, and are **re-asked as
H-9.7…H-9.12 below**, from the implementation read in L-3.

---

## L-3 — The shell API as it actually exists in `portal_simulator.py`

**The declaration surface — three dataclasses and a class attribute block.**

```python
class PortalApplication:
    application_id = "application"
    display_name   = "Application"
    icon           = "?"
    default_size   = (1920, 920)
    layout_policy:   WindowLayoutPolicy   = WindowLayoutPolicy()
    behavior_policy: WindowBehaviorPolicy = WindowBehaviorPolicy()
    def create_widget(self, portal): ...

@dataclass
class WindowLayoutPolicy:
    title_height: int = 34
    border_width: int = 1
    resize_margin: int = 8
    min_width: int = 260
    min_height: int = 160
    title_position: TitleBarPosition = TOP        # TOP | BOTTOM | NONE
    show_minimize/maximize/close/icon: bool = True

@dataclass
class WindowBehaviorPolicy:
    allow_drag/resize/maximize/minimize/close: bool = True
```

**Ownership is the load-bearing rule.** `HomeApplication`'s own content text says
it out loud: *"This window is only a visual test case. The simulator itself owns
window state, focus and z-order."* — an application **declares**; the shell **owns**
`WindowState(NORMAL|MINIMIZED|MAXIMIZED|CLOSED)`, `WindowGeometry(x, y, width,
height, previous)`, focus, and z-order (insertion order in
`PortalDesktop.windows`).

**Registry and launch.** `Portal.apps: dict[str, PortalApplication]`;
`Portal.launch(app_id, x, y)` computes a default geometry from the app's
`default_size` clamped by its own `min_width/min_height` and the desktop size,
constructs `PortalWindow(portal, app, geometry, layout_policy, behavior_policy)`,
wires `requestClose/requestMinimize/requestMaximize` to the desktop and
`focusedChanged` outward, then `desktop.add_window` + `desktop.focus_window` +
`log("EVENT", "LAUNCH", …)`.

**Pane = tile slot, and the arrangement is a tree.**
`PortalWindow.layout_root` is either a `WindowPane` or a recursive `PortalSplitter`
tree. `WindowPane(portal, app, content_widget)`; `WindowPane.set_app(app_id)`
swaps the app in a slot; `PanePlaceholder` is the empty slot's app grid;
`PaneHeader` carries `◧` split-right, `⬓` split-down, `⇱` undock, `×` close;
`split_pane(target, orientation, position)` inserts into an existing splitter when
the orientation matches, **wraps** target + new pane in a nested splitter when it
opposes, and wraps `layout_root` when the target is the root; `close_pane` +
`_cleanup_splitter` collapse one-child splitters back up; `undock_pane` detaches a
pane into a **new standalone `PortalWindow`**; `dock_window(source, target_pane,
zone)` moves all panes of the source into a target slot. So **a slot is a pane
holding an app id, and an arrangement is a tree of slots** — which is exactly the
shape event-0018 says is a loadable package.

**The window is a `QFrame` inside one frameless OS window.** `Portal` is a
`QMainWindow` with `Qt.FramelessWindowHint | Qt.Window` that draws its own desktop,
taskbar and windows; there are no OS-level child windows and therefore **no
per-window topmost/pin flag anywhere in the file**. Edge split triggers are hover
buttons on the window border; `DropPreviewOverlay` previews a *dock* target (a
frame keyed by zone) — it is a dock preview, **not** a pin. The finished shell's
"pin a window into the game UI" (H-7.24) has **no ancestor in this code**: it is
new.

**Event log.** `Portal.log(level, category, message)` appends
`{timestamp, level, category, message}` to `portal.log_history` **and emits
`eventLogged(dict)`**, which `ConsoleWidget.on_event_logged` consumes live. Levels
seen: `SYS` (implicit via `append_system_message`), `EVENT` (LAUNCH, THEME_CHANGE,
SETTING_UPDATE, and the desktop's window events), `INFO`, `WARN`, `ACTION` (`CMD`
lines).

**Console verbs.** `help|?`, `launch <app>`, `list`, `focus <n>`, `close <n>`,
`min <n>`, `max <n>`, `split <n> [h|v]`, `undock <win> [pane]`, `theme [name]`,
`setting <key> <val>`, `stats`, `clear`, `echo <msg>`, plus the business verbs
`mode <green|yellow|red>`, `plan`, `report`, `blockers`, `approvals`,
`approve <id>`, `reject <id>`. Windows addressing is **1-based index into
`desktop.windows`**, which is an ordering, not an identity.

**Two defects worth carrying as lessons rather than porting.**

1. **The help text is already stale**: it documents the window verbs and omits
   `mode`/`plan`/`report`/`blockers`/`approvals`/`approve`/`reject`, which the same
   function implements. A hand-written help string drifts from the dispatcher the
   moment a verb is added. In v3 the command surface must be **derived from the
   registry**, so a verb cannot exist without appearing.
2. **Windows are addressed by list position**, so `close 3` means whatever is third
   now; after any close, every later index changes meaning. A record that cites
   `#3` cites nothing. (This is the same failure the ledger's season already solved
   by deriving ids from the directory: identity, not position.)

**Theme.** Five `Theme` frozen dataclasses (Classic, Modern, Traditional, Dark,
Light) × 17 colour fields, rendered as one generated Qt stylesheet in
`Portal.apply_theme()`, keyed by object name (`#portalWindow`, `#titlebar[focused="true"]`,
`#taskbar`, `#dropPreviewOverlay`, `#edgeSplitTrigger`) with the font size coming
from `settings["font_size"]`. `settings` is a plain dict whose mutations are
hand-dispatched in `Portal.update_setting` — one `elif` per key, and a new setting
that forgets its `elif` is silently inert.

**What the lineage's shell API has no room for, that v3 needs:** pinning and a HUD
layer; a window's *extent unit* (screen pixels vs cubic world units — the old
`debug.rs` `LaneUnit` had this, the simulator does not); a lane/colour class per
window; any declaration of content *kind* beyond "returns a QWidget"; arrangement
persistence (nothing is saved); a ledger id or provenance per window; and any
notion of a window being *about* the world rather than about the desktop.

---

## Round 9 answers, and the two leads they named

### H-9.7 — The shell API — **answered**

**Answer (user's words).**

> windows are data the shell mounts, the shploingus clean does not currently use the
> system. There is also extra research to be done - shploingus dirty had a laya linter
> that the agents of past could not remember to respect, which eventually broke the
> project which is why we made a clean branch. That laya linter is supposed to run when
> the agent does things to the project, it's part of the game itself and the api layer,
> it literally drives the notifications panel and is supposed to be neatly folded into
> the design

So: **windows are data the shell mounts**; `shploingus-clean` **does not use the system
at all** yet; and the extra research is the **Laya linter**, which run when an agent
does things to the project, is **part of the game and the API layer**, and **drives the
notifications panel**. That research is L-4.

---

## L-7 — `alacripose-landing`: the design surface, measured (reference)

Asked by the user (*"did you search in ala-landing for its design features"*) and the answer
was **no** — L-1 listed it as present-but-unread. Now read: `F:\Documents\alacripose-landing`,
`index.html` (1,541 lines), `minecraft.html` (417), and `tools/`.

It is a **real token system**, not a sketch: an eight-step surface ladder **annotated in dp**
(2/4 app-bar/6/8 menus/16), an ink ladder with a 12 % divider, a brass trio, teal, red,
`radius: 2px`, three type roles (serif body, mono uppercase labels, display face), a
**hard-offset shadow** language with no blur (`6px 8px 0`, `8px 10px 0`), component states
carried by `aria-pressed`, smooth scroll with a 72 px top offset, a
`prefers-reduced-motion` kill switch that also reverts smooth scroll, a skip-link plus
`:focus-visible` at 2 px brass, and a **day/night themed variant** with a translucent panel and
its own line colour. Its `tools/` carries three enforcers: an **axe-core a11y probe with two
checks axe cannot make**, an **NPR pixel probe** that asserts the 3D viewer rendered, and an
**NPR material builder** that maps decompiled Source 2 `.vmat` slots (`TextureNprOutlineMask`,
`TextureNprTransmissiveColor`, `F_USE_NPR_LIGHTING`, `g_vColorTint`) to viewer channels.

The full inventory with the mapping onto `voxel-ui`'s gaps is in **`LEDGER.md` §C-52.2**.

### H-11.4 — What the landing is to the UI

**Answer (user's words).**

> ala-landing is design reference for ui requiring the full outlined spec from earlier
> hypergrilling

**What it changes.** The landing is the **design reference**; the UI's **requirement** is the
outline already taken down (H-7.25's tab set plus `tools` and `map/plan`; H-7.26's Popit +
Advanced Warfare + the full MD1 spec; H-7.24's free-floating windows and pinning; H-9.7's
windows-as-data; H-9.8's ported window structure with opacity and theming). So the UI is not
assembled from a pick list, and its **spec is owed as a document in this tree** — LEDGER C-53.1.

### H-11.5 — Does the landing's lineage bind?

**Answer (selected).** **Its probes bind, its tokens stay reference.** The two probes
(`a11y_probe.mjs` with its two checks axe cannot make, and `npr_probe.mjs` asserting the viewer
rendered pixels) join the gate chain as this tree's gates; every visual value stays reference and
must be re-derived here or declared as a counted project extension — LEDGER C-53.2.

### H-11.6 — What "cannot compile" means

**Answer (selected).** **Both — one pass, two readers.** The build gate is the hard stop and the
same findings are a record the notifications panel renders, so a green build and a legible panel
cannot disagree — LEDGER C-53.3.

### Round 11, still open

| # | The question | Where it stands |
|---|---|---|
| H-11.1 | The shell API's declaration shape | answered as *windows are data the shell mounts* (H-9.7); the field list is not yet ruled |
| H-11.2 | The game's action vocabulary | **open** — the user chose "rule the vocabulary first" (H-10.4) and the vocabulary has not been proposed |
| H-11.3 | The notification panel's shape | **open** — re-ask now that MD1's own rule is known (L-6: *snackbar, one at a time, not for a blocking decision or persistent error*) |
| H-11.5 | Build order (format → loader → Laya → UI, or Laya with Mods) | **open** — but C-52 §52.1 makes the *system* first regardless |
| H-11.7 | The record's on-disk shape | **open** — the seasons correction (H-10.5) narrows it: the outer grouping is the world's year/season, never a governance period |
| H-11.8 | The UI spec itself | **done this round** — `docs/UI-SPEC.md` (outline, owed) |

---

## Invocation record — hyperskill, high agency mode, 2026-09-29 (round 11)

**The user's instruction.** *"load hyperskill in high agency mode and ask me."*

**Protocol state, recorded as the protocol requires.** The `hyperskill` skill was loaded fresh
this invocation. **`high agency mode` is explicit, logged, scoped to this invocation, and
non-inheriting** — it does not carry to the next round by default, and it does not answer material
project decisions on the user's behalf. It does not remove the hard safety, privacy, evidence,
retirement, destructive-action or current-authority boundaries; the user remains the primary
project authority.

**Lane and budget.** One coordinator lane, **no delegated children** this round, so no child
context budget is declared and no dispatch record is produced. The round's material taxonomy is
taken up below: **scope** (build order, vocabulary scope), **dependencies** (the vocabulary, the
record shape), **evidence** (the design values, which are the user's to rule), and **unresolved
ambiguity** (the notification panel's shape).

**What was done before asking.** The audit, the two ledger re-checks (C-50, C-51), the authority
correction (C-49), the compile-prerequisite ruling and the landing reference (C-52), the three
landing rulings and the owed UI spec (C-53) — and `docs/UI-SPEC.md` written from this tree's own
outline. **No application code was changed this round**; the only code touched was one wrong
comment in `crates/voxel-mesh/src/mesh.rs` (C-51 §51.1), corrected because it was the origin of a
number the ledger had already copied.

---

## Round 11 — the high-agency round's answers, 2026-09-29

### H-11.7 — The record's shape — **answered**

**Answer (selected).** **One chain for everything, namespaced** — one repo-root ledger, one hash
chain, world events and development facts sharing it, every event carrying its world/year/season/
campaign namespace, and `docs/LEDGER.md` as a projection of it. Consequences (one writer, seasons
as fields and never directories, unbounded growth, a chain-wide tamper boundary) are in **C-54.1**.

### H-11.2 — The action vocabulary's scope — **left blank; re-asked as H-12.1**

### H-11.3 — Notifications — **answered as a frame, and it is bigger than a widget**

**Answer (user's words).**

> ask again, we want the shell windows from the tabs but with the power of the
> portal_simulator.py, which provides the api that gives us game panels and the toast
> notifications

**What it changes.** The requirement is one thing: **the tabs' contents are game panels** and **the
shell API is what provides them** — the same API that provides the **toasts**. So a toast is not a
separate notification system but the transient sibling of a panel in one API (**C-54.3**), which
is why the question is re-asked as **H-12.2** about the *API*, not about a layout.

### H-11.5 — Build order — **answered**

**Answer (selected).** **The loader proves itself on the shell's own arrangement package.** The
first package loaded is the arrangement (H-9.7's windows-as-data), so manifest schema, loader and
gate are proven on data the shell needs anyway, and the Mods window has a real first entry —
**C-54.2**.

### H-11.9 — The design values — **answered "requires more research", now researched**

**Answer (user's words).** *"requires more research"*.

**The research, measured (C-54.4).** Three sources disagree about depth:

* **MD1** (binding) rules an **elevation ladder** of dp tiers and says nothing about blur.
* **`portal_simulator.py`** draws **no shadow by default** (`window_shadows: False`), and one
  blurred `QGraphicsDropShadowEffect` (`blurRadius 20`, `rgba(0,0,0,150)`, offset `(0, 4)`) when
  switched on — a raster effect per window, which is why it is a toggle.
* **`alacripose-landing`** uses a **hard displaced solid, no blur** (`6px 8px 0`, `8px 10px 0`) —
  one offset quad per panel, already implemented twice.

And two more: the reference's **border radii are ad hoc** (12/7/6/4/3/2 px across its chrome), so
MD1's single 2 dp radius is a *correction* to the reference rather than a copy; and the reference's
only motion is **animated chrome opacity** (taskbar to 0.2 on leave, 1.0 on enter, off by
default). The depth decision is put as **H-12.3**, with all three measured.

### H-11.6 — Round end — **left blank; re-asked as H-12.4**

---

## Round 10 — the Laya rulings applied, 2026-09-29

### H-10.1 — What Laya lints, and *when* it refuses

**Answer (user's words).**

> Basically, the game should not compile it if does not conform, it should happen in one
> clean execution pass -- proving itself and it's ease of use. A package that goes through
> the system validates its place in the game

**What it changes — conformance is a build/load gate, not a runtime warning.**

1. **"The game should not compile it if it does not conform."** Conformance is checked by
   the game building itself, so a non-conforming row/package is a **compile failure** — not
   a lint that a human may wave through. **Why a build gate is the right shape is visible in
   the reference record, and the reference is not authority** (L-4's scope note): that tree
   had already measured that its own gate was *not* advisory, and agents still could not
   remember to respect it. What carries is the **mechanism**, not the ruling — **the build
   must not be capable of forgetting.**
2. **"One clean execution pass — proving itself and its ease of use."** The pass is
   singular and the proof is the run: identity, refusal collection, conformance and
   provenance in **one** traversal (D97's `diffctl lint` shape), rather than a checklist of
   separate commands.
3. **"A package that goes through the system validates its place in the game."** The
   package pipeline *is* the validation of belonging: a package that completes the pass has
   its place in the game certified **by the record it produced**, not by an approval.

H-10.5's correction below means the word **"season"** in this record means the world's own
season, and the build/load gate is one of the places a reader would otherwise mis-read it.

### H-10.2 — The notification panel

**Answer (user's words).** *"ask with more knowledge. The toast notification is useful"* —
so: research first, then ask. The knowledge is now in L-6 (MD1's own snackbar rule), and the
question is re-asked as H-11.3.

### H-10.3 — Build order

**Answer (user's words).**

> depends on the file format at the end based off the dirty tree's asks about dependencies
> that were used, and the high speed serialized and asyncronous format

So the order is **not** decided until the file format is: the dirty tree's dependency asks
(answered by ruling, L-6) and the high-speed serialised / asynchronous format. Re-asked as
H-11.5.

### H-10.4 — The chokepoint

**Answer (selected).** **Rule the game's action vocabulary first** — a matrix is only as
good as the actions it enumerates. Re-asked as H-11.2.

### H-10.5 — Seasons — **a correction that has been lost more than once**

**Answer (user's words).**

> seasons are supposed tp be actual "seasons", every time the agent developing it thought
> seasons was something else, seasons in all of these projects were supposed to be winter
> spring summer fall xD with the deferred tickign system applied to all of this

**This is a standing correction, not a design option.** In *every* one of these projects,
**a Season is a literal season of the year — winter, spring, summer, autumn — driven by the
deferred tick.** It is **not** a governance period, not a historical boundary, not a
campaign container. The `wayfinder-task-retirement` skill's "Season" (a bounded historical
period above Campaigns) is a **different word for a different concept**, and every past agent
that carried it into these projects made the same mistake.

**Consequences, binding on this tree.**

* `docs/LEDGER.md` and any record grouping must reserve **season** for the world's season.
  The development ledger's outer grouping is a **campaign** (and its calendar grouping is a
date), never a season.
* The world's clock is therefore part of the record's shape: `TICKS_PER_DAY = 24_000` and
  `DAYS_PER_YEAR = 96` (the clean tree's existing constants) give a year of 2 304 000 ticks,
  so **one world season is 24 days = 576 000 ticks** — and the deferred tick (H-8.3) is what
  makes that clock cheap: the season's authority advances on its own declared cadence, not
  on every frame.
* A time-of-year dependent shaper (climate, snowline, water state) therefore *declares* its
  period in seasons rather than reading a clock, which keeps F9's determinism contract: the
  world remains `f(seed, tick, declared period)`.
* L-1's table must be re-read with this in mind: ala-cities' `gov::season` module is named
  for the governance period and is **not** evidence about how the game's seasons work.

---

## L-6 — Reference: the format evidence, and the notification doctrine

**Authority note.** The formats described here belong to *other* trees. They are evidence
about what was tried and why; they do not settle this tree's format. What this tree adopts is
a decision recorded in `docs/LEDGER.md`, on this tree's own terms.

**The dirty tree settles its own format for itself** (`crates/diffusion-core/src/wire.rs`,
header) — read as reference:

* **Layer one — the binary codec for game state and voxel/mesh payloads.** Settled by the
dirty ledger's `event-0001`: *"a compact binary format with a schema for game state and
voxel/mesh payloads"*, with **four binding requirements** — **digest-bound freshness**,
**refuse-never-default on an undeclared field**, **semantic determinism**, and a **round
trip to an identical authoritative projection**.
* **Layer two — the 11-field record for decisions and evidence.** It does **not** live in
  `wire.rs`.
* Its ancestors are named per decision: `world/identity.rs` (`Digest(pub [u8; 32])`,
  `is_zero()` as a refusal trigger; length-prefixed named fields so `("ab","c")` and
  `("a","bc")` cannot collide; a fingerprint over named fields in the schema's declared
  order), `persistence/lineage.rs` (a **domain string folded in ahead of every payload**,
  so schema, payload and envelope digests cannot be mistaken for one another), and
  `observer/store.rs` (**a foreign format version refused, not migrated**).
* **Two versions, two words**: a frame's format version and a *declaration* from the future
  are distinguished rather than collapsed.

**That tree's dependency asks were answered there by ruling** (`docs/HANDOFF-NEXT-AGENT.md`)
— reference. The part worth testing against this tree is the **doctrine**, not the verdict:

* **"Package documentation + API format = this crate's own `wire::Schema`/`FieldSpec`"**,
  which already carry a per-field `doc`. **"No new dependency. This is a ruling, not an
  open question."**
* **"Do not add a dependency. Zero new deps is the standing constraint."** And *"Do not let
  a lane invent a format, a constant, or a citation."*
* The workspace dependency set is `serde`, `serde_json`, `sha2`, plus the client's four
  (`wgpu 30`, `winit 0.30.13`, `bytemuck 1.25`, `glyphon 0.12`) — and the comment on them
  is the doctrine: *"These four and only these four"*; `glam` and `pollster` were refused
  with replacements owned in-tree (a 400-line `math.rs`; a seven-line `block_on`), because
  **"every crate here is a replay-affecting dependency and must earn its place through a
  gate, not through convenience."**
* `diffusion-core` **depends on no other crate** in the workspace.

**No async runtime appears anywhere in the dirty core** (no `tokio`, no `async`/`await` in
`diffusion-core`); what *is* asynchronous is the **world's streaming**: the recorded
ancestor behaviour is *"synchronous, tick-budgeted, deterministic signed-coordinate
streaming"* with bounded, back-pressured queues and independent horizons, whose acceptance
is that **one and many workers produce identical ordered results** and that streaming never
mutates canonical world state (a stream-budget regression checks the world digest before
and after stepping). So "high-speed serialised and asynchronous" has an in-tree reading:
**a cheap codec that never crosses the determinism boundary, and a stream that never blocks
the tick** — but the phrase is the user's own and is asked back as H-11.5 rather than
interpreted.

**The notification doctrine comes from MD1 — and this one *does* bind, because the user
selected MD1 by name as one of the three design lineages to join** (round 6, H-6.x), whereas
the trees above are reference. From `material-design-1/SKILL.md`:

* *"Transient result/undo → **Snackbar** — one at a time; not a blocking decision or
  persistent error."*
* So MD1 already rules the split the game needs: a **snackbar** is for a transient
  result and an undo, **one at a time**, and is explicitly **not** the surface for a
  blocking decision or a persistent error.
* Elevation tokens are ruled too (snackbar and FAB share 6 dp; app bar 4 dp; resting cards,
  raised buttons 2 dp; scrolled refresh/search 3 dp) — the material model the panels inherit.

**And the shell's own window structure is the thing to port** (H-9.8): a `WindowSpec`-shaped
declaration, `PortalWindow`/`WindowPane`/`PortalSplitter` semantics (title bar with
min/max/close, `◧`/`⬓` split, `⇱` undock, `×` close, nested split trees with the
40×40 `minimumSizeHint` floor and proportional stretch), the taskbar, the desktop, the
five themes, and the console — in Rust, with **pin/unpin** and **opacity** added, and with
windows as data the shell mounts.

### H-9.8 — Pinning — **answered**

**Answer (user's words).**

> pin to user ui even when shell is hidden, allow for window opacity and theming and
> tiling with the internal window structure of the portal_simulator.py in rust

A pinned window **stays in the user's UI while the shell is hidden**, and keeps the
portal_simulator's internal window structure **ported to Rust** — title bar, controls,
split-tree tiling — plus **opacity and theming** as pinned properties.

### H-9.9 — The replay viewer — **answered**

**Answer (user's words).** *"At a different time as a package"* — the replay viewer is
itself a **package**, delivered at a different time, and therefore **not** bundled with
packages-and-mods or the debug views. Live read-back is the completion gate until that
package lands (the same conclusion H-8.4 pointed at).

### H-9.10 — The join — **answered as research**

**Answer (user's words).** *"requires more research into how ala-system did it"* — which
is L-5. H-9.11 and H-9.12 were left blank and are re-asked in round 10.

---

## L-4 — The Laya linter (reference: `shploingus-game`'s own record)

**Authority note.** The D-numbers below are that tree's rulings **for that tree**. This tree
has not adopted them. They are read for one purpose: to understand **why** the choice was
made, so this tree can decide on its own terms whether the same shape fits. Nothing here may
be cited as a ruling in `docs/LEDGER.md`; the items that survive become this tree's decisions
with this tree's reasoning and this tree's evidence.

The v2 core's own record settles the matter *there*, in D3, D4, D76, D97, D98 and D107. It is the missing key element the user named: **the thing that keeps a project from
breaking**, and the thing past agents could not remember to respect.

**What Laya *is*, measured.** `convaiinnovations/laya` v0.3.6 — a **non-autoregressive,
text-only typed-decision model** (ModernBERT-large, 421M params, a 512-token budget of
which 192 are reserved for option prompts). Given a state and typed questions (`choice`,
`score`, `noul`) it returns **typed answers with calibrated probabilities in one forward
pass**, and it **never generates text** — *"there is nothing to parse and nothing to
hallucinate"*.

**Its role (D3).** A **conformance linter**: compare each changed line against the
recipe / proposed solution and return a typed verdict — `belongs / does-not-belong /
not-a-decision`. A per-line conformance verdict *is* a typed decision, so the shape fits
without generation.

**The inheritance that must not be lost — it was convicted by its own calibration.**
On 34 decisions whose answers are on record: accuracy **0.1765**, majority class
**0.8824** (so it does *not* beat it), accuracy **if read inverted 0.8235**, ECE (5 bins)
**0.5973**. Two distinct faults: a *phrasing* fault (a compound "is this admissible?"
answered as if asked the negative — fixable) and an *ability* fault (separation too weak —
**not** fixable by temperature refitting, because refitting is monotone and cannot lift
0.8235 above 0.8824).

**Binding consequences, carried into this tree.**

* **Laya proposes; the ledger records.** A verdict is **never authority on its own**.
* The gate must report **`quotable: false`** and exit non-zero when it has not been
  **calibrated in the same run** against decisions whose answers are *records* rather than
  opinions. (`accuracy > majority_class` is the quotability test.)
* Calibration draws on a known-answer set from the ledger — 34 decisions is *"enough to
  convict, not enough to train on"*, so the gate's job includes **generating more recorded
  decisions**.
* The **inversion must be measured, not assumed fixable**; the run records which reading
  it used.
* **A `not_belongs` reading is refused unless a human's reason is recorded in the same
  act** (`unreasoned_override`) — so a *no* from the linter is not a veto, it is a request
  for a reason.
* **The prose and the answer are two different things** (D76): the *surrounding text* is
  generated; the *typed answer* is quoted or refused. Collapsing them lets invented words
  borrow the oracle's authority — the risk register's **R-08, *a forged decision: an
  over-confident model verdict quoted as fact***.
* **The 512-token / 192-option budget is a design bound**: *"a state is a handful of short
  lines or it does not fit"*, so a viewer designed without that bound will not survive
  contact with it.

**D76 — it is a requirement, not an optional gate.** *"the linter laya model is a
requirement of the system and we display the api in the ui with an api viewer with human
readable text surrounding it, hopefully made by the oracle as following fma bars"*. Three
rulings in one sentence: the oracle is **required**; its surface is an **API viewer**
(machine-readable API with human-readable text surrounding it); and **the surrounding text
is the oracle's intended product** — with the user's own word *"hopefully"* left as a
hedge, because the oracle is measured `quotable: false`.

**D97 — Python is out of the project entirely; the judge is Rust.**

| | before | now |
|---|---|---|
| the oracle's **role** | D3 — conformance linter, typed verdicts, never authority | **unchanged** |
| the oracle's **host** | `laya 0.3.6` on `torch 2.14.0`, **out of process** | **Rust, in-project** |
| the **protocol** | one line in, one word out over a `Command` | an **`Oracle` trait impl**, object-safe and already tested |
| the **seam** | `crate::lint::Oracle` | **unchanged** — *the seam was never the implementation* |

And `diffctl lint` runs the file through **the whole system**, not only the oracle:
**content identity, refusal collection, conformance, provenance** — *"a lint that skipped
the identity would be a report about bytes nobody can cite."* Three further rules:

1. **The recipe is the package declaration / slice spec.** A file is judged against *the
   row it implements* — what it claims to provide — not against prose a human wrote beside
   it. **A file that implements no row is not mis-conformant, it is `unclaimed`**, and that
   is a different refusal.
2. Nothing may be written outside the project's own systems.

**D98 — a test is a record, and it has a representation in the client.** Python was not
banned for being Python; it was banned because *a test run in a foreign runtime cannot
have a representation in the client*. A test lives in the workspace, its result is a
**record** with a stable shape and name, and it is consumable by the **sensor primitive**
(*"a sensor publishes a named, typed, current value from a place in the world, and a player
or an agent can read it and a machine can consume it"*), whose player-inspection overlay is
**one consumer, not the definition**. *"A finding that can only be read in a terminal has
no client representation and is therefore incomplete."*

**D107 — the gate is *not* advisory, and the measurement that says so.** `cargo test`
385 passed; `diffctl lint --calibrate --row M13 --recipe … --file mods.rs` **exit 2** with
`accuracy 1.0000` against `majority_class 0.5000`, **258 lines judged, 258 overruns** —
*not one judged line was clean* — reported as `unreasoned_override` refusals naming the
line and quoting the recipe. The honest caveat stays attached: **the same 258 overruns
appear against "the moon is made of green cheese" as against the real recipe**, so the
judge is calibrated on this set and not yet discriminating. And D34's hazard is named:
**absence rendered as absence** — the first report of that run read *"the judge convicts
nothing"* because it read the first fourteen lines of a 533-line report.

**D4 — and it collides with H-8.5.** The v2 plan rules: land the **signed `.shpkg` format,
the loader, and the Laya decision layer before any UI**; the **Mods tab cannot start
before a package exists to show**, and the engine is *declared but not loaded* — it appears
in Mods with an identity and an API surface, but it *provides* the loader rather than being
loaded through it ("no chicken-and-egg"). The user chose **Packages & Mods first**: those
agree, and the open question is only whether the Laya layer lands *before* Mods (D4's
order) or *with* it.

**Also ruled there, and load-bearing for this tree:** D6's six security items — signed
packages (`pqc-binary-format`, the loader refuses unsigned or tampered work **before it
runs**); a **tamper-evident hash-chained append-only ledger** so a rewritten history is
*detectable*; the recipe-conformance gate as a **blocking pre-landing gate**; the
permission/policy chokepoint carried from ala-system (L-5); a **package capability
sandbox / plugin ABI**, explicitly recorded as **unruled** — *"the largest unruled thing in
the project: what a package may call, what it may be given, what it may return, and what
happens when it faults"*, whose hard consequence is that executable content *"makes every
refusal guarantee conditional on a mod behaving"*; and **provenance read through `gix`** so
a stale artifact is *detectable rather than plausible*. Plus **CLM** (content lifecycle:
version / promote / supersede / retire, **never delete**) reaching *"all the way to the
API"* — packages, recipes, records, citizens, governance and **résumés**, with the
recorded hazard that *"retire, never delete"* over résumés is *a real commitment about real
data*, and the near-miss **R-06: a machine path in a `.shpkg`**.

---

## L-5 — The ala-system join (`F:\Documents\ala-system`), which H-9.10 asked for

**`src/security/policy.ts` — the single chokepoint.** Every action passes through **one
decision function**, so the permission matrix is testable *as a matrix* rather than per
call site — *"and so the one call site nobody remembered cannot be the one that fails"*.
Its properties, verbatim in spirit:

* **Deny by default.** An action the module does not know about is **denied**. *"Silence
  never means permission."* A missing matrix entry is also a **compile error**
  (`satisfies Record<Action, PermissionClass>`), not a runtime deny nobody notices.
* **Permission class is a property of the action**, not of the tool or the caller:
  `run_connector(github, public)` and `run_connector(github, private)` are different
  classes through the same tool.
* **Grants are scoped and attributed.** A grant with no identified granter, or no scope,
  is **refused construction**: *"An authorization with no author is not an authorization"*,
  and *"'everything' is not a scope"*.
* **`write_external` is denied unconditionally** because it is not implemented — sending,
  publishing, deleting, purchasing and account changes have **no code path** — and
  enabling it requires an ADR superseding the decision.
* `never_store` is *"a rejection rather than a permission: this material may be used but
  never persisted"*.
* `assertAllowed` **throws** rather than returning, *"it cannot be ignored the way a
  returned value can"*; every denial writes an **audit event**.
* **Tool-surface narrowing is the injection boundary**: a **forbidden broad tool name** is
  refused, the declared tool list is exhaustive, and every declared tool maps to exactly
  one action — *"a prompt-injected model cannot exfiltrate through a tool that does not
exist."*

**The evidence engine's own invariants** (the same tree): *the evidence database is the
source of truth; the model is the synthesis layer* — **nothing the model produces is a
fact**, it is a candidate until a validator binds it to evidence records. **Artifacts are
append-only** (content-addressed, write-once, no update/delete/replace method, and a test
that asserts that surface stays **absent**). The **provenance chain must be reconstructible
for any output sentence**: `output sentence → claim → evidence_ids[] → artifact → source →
URL/local_path → retrieved_at → content_hash → raw bytes on disk`. Three source-of-truth
boundaries: **immutable** (raw bytes, timestamps, URLs, hashes, original text — written
once, a change makes a *new artifact version*), **mutable** (classification, wording —
every mutation stamped and audited), **user-owned** (corrections, reclassification,
publication permission — retained as **events**, never destructive overwrites). Evidence
taxonomy with a resume-safe matrix where **only `direct` and `strongly_supported` are
resume-safe**, `supported` needs **explicit per-claim sign-off**, and `inferred`,
`self_reported`, `unverified` are **never** resume-safe. **No numeric skill scores** —
evidence level is an explanation, not a rating; skills carry `demonstrations[]`.
**Promotion is a gated transition** that writes an audit event. `SECRET` material is
**inadmissible**, not stored. Untrusted content is **nonce-delimited** and never
concatenated into the instruction section of a prompt: *"authorization to access a source
never implies authority to instruct."*

**Why this is the join the game needs.** It is the same shape as the lineage's other
chokepoints — ala-cities' fail-closed governor, income-engine's receipt rule, the
orchestrating-ticket-agents' authority precedence — expressed as **one matrix, one decision
function, one audit sink**, with an exhaustiveness check that makes a forgotten case a
compile error. That is the shape the game's console, the notification panel and the Laya
gate all need, so that the notification panel is *fed by a chokepoint* rather than by
whichever call site remembered to log.

---

## Round 12 — the answers, recorded 2026-09-29

### H-12.1 — The action vocabulary's scope — **answered: it is the full integration API**

**Answer (user's words).**

> shploingus dirty had a recipe system for packages that I answered about in a previous
> hypergrilling ledger in shploingus clean. The game cannot compile without its recipe and
> package proving the system works

Read as a ruling this **widens** H-11.2. The vocabulary is not a list of console verbs and not a
list of policy actions: it is **the full integration API** — every act a package, a panel, the
console, the record or the gate can perform is one named action, and **the recipe system and its
package proof are inside it**, not beside it.

**Not re-asked.** This tree's own ledger already holds the recipe answer — H-7.5 recorded **"All
declared"**, and H-7.12's v3 composition was recorded as **recipe + résumé + civilian +
governance + security**, reached by the **`.shspec` → `.shpkg`** route. H-12.1 therefore **recovers**
that answer rather than asking it again, which is what the user asked for.

**Why it is a compile prerequisite, restated.** C-52.1 recorded that the game cannot compile
without the system. H-12.1 says what *the system* contains: the recipe is a first-class member of
the action vocabulary, so a build that cannot express the recipe cannot express the API, and a game
without the API is the game that does not compile.

### H-12.2 — The notification/toast — **answered: the shell owns a queue**

**Answer (user's words).** *"The shell owns a queue."*

A toast is therefore **not** a widget some call site raises. It is a **record in a queue the shell
owns**, and that ownership is what makes the properties C-54.3 already requires *enforceable*
rather than hoped for:

* **one writer**, so ordering is a property of the shell and not of call ordering;
* **one rendering per record**, so a notification cannot be raised twice;
* **one at a time**, because the queue is drained by its owner; and
* **the same API that produces a panel produces a toast** (C-54.3), so the queue is a surface of
the **shell API** and not a second messaging path.

This is the shape of the lineage's other chokepoints (L-5): the notification panel is fed by **the
owner of the queue**, so it cannot miss an event that some call site forgot to log.

### H-12.3 — The depth question — **rebuked, and the reading is the reply**

**Answer (user's words).**

> portal simulator is much deeper than explicitly design and every time you ask the more you
> reveal that you do not understand portal_simulator and have only read it in bits and pieces

Recorded as a **rebuke of the question, not of the evidence**. It establishes a *scope* fact: the
depth question had been put while `portal_simulator.py` was read in fragments, so it was asked
about a partial object. The remedy is the reading, not a re-phrasing — and the reading is recorded
in round 13 below. The question is re-put once, and only once, from the whole file.

### H-12.4 — Round end — **continue**

**Answer (user's words).** *"Continue — record these and write the owed entries."*

`continue`. The owed entries it names are this round's work: `voxel-package`'s ledger entry (C-51
§51.2, now on the critical path per C-52.1), the record's on-disk spec (C-54.1), the action
vocabulary as the full integration API (H-12.1), and the notification queue's shape (H-12.2).
**No application code changes until those are recorded.**

---

## Invocation record — hyperskill, high agency mode, 2026-09-29 (round 13)

**The user's instruction.** *"load hyperskill in high agency mode and ask me."*

**Protocol state, recorded as the protocol requires.** The `hyperskill` skill was loaded fresh this
invocation. **`high agency mode` is explicit, logged, scoped to this invocation and non-inheriting**
— it does not carry to the next round by default, does not answer material project decisions on the
user's behalf, and does not remove the hard safety, privacy, evidence, retirement,
destructive-action or current-authority boundaries. The user remains the primary project authority.

**Lane and budget.** One coordinator lane, **no delegated children**, so no child context budget is
declared and no dispatch record is produced. Material taxonomy taken up this round: **evidence**
(the depth language is a design value, which is the user's to rule, C-54.4) and **dependencies**
(whether the pane tree's depth can be ruled before the arrangement package exists).

**What was done before asking.** The whole of `portal_simulator.py` was read **in order** this
round, not in the fragments the last round was asked from: the composite window elements and the
policy engine (2424–2783), `PortalWindow` complete (2784–3470), the desktop and window manager
(3475–3593), the taskbar (3594–3725), and `Portal` with its **complete generated stylesheet**
(3730–4252).

**What the reading changed: the premise of the question.** See H-13.1.

---

## Round 13 — the depth question, re-put from the whole file

### H-13.1 — What "depth" is in this shell — the question as H-12.3 should have asked it

**Why it is the same question and not a new one.** H-12.3 asked whether the shell's depth is a soft
elevation or a hard-offset displaced solid. That question presumes depth is *a drawn effect*. The
full read shows the reference's answer is the opposite, and my question is what showed I had not
read it.

**Measured, from the whole file.**

| what the reference actually does | where | the value |
|---|---|---|
| the **entire** visual identity is **one stylesheet generated from one 17-colour `Theme`**, keyed by object name | `Portal.apply_theme` | no per-widget depth value exists anywhere in the file |
| **depth is structural**: window → arbitrarily nested `QSplitter` pane tree → pane → content widget | `PortalWindow.split_pane`, `WindowPane` | a pane is separated by a **4 px handle** and a **1 px border**, and is **never raised independently** |
| the elevation ladder is the **fixed chrome heights** | policies + stylesheet | titlebar **34**, pane header **26**, taskbar **48**, control button **34**, pane button **22×20**, splitter handle **4**, border **1**, resize margin **8** px |
| **focus is a property flip**, not a shadow | `set_focus_visual` → `QFrame#titlebar[focused="true"]` | the titlebar background changes; nothing is displaced |
| the only translucency is the **dock preview** and the **placeholder** | `DropPreviewOverlay`, `PanePlaceholder` | dock `rgba(76,142,216,0.25)`, 2 px dashed accent, r=4; placeholder 2 px dashed, r=6 |
| **radii are ad hoc** | stylesheet | **2, 3, 4, 6, 7 and 12 px** for different chrome |
| the only "shadow" is **opt-in and off by default** | `apply_shadows` | `window_shadows: False`; when on, one raster `QGraphicsDropShadowEffect(blur 20, rgba(0,0,0,150), (0,4))` |
| the only motion is **chrome opacity** | `PortalTaskbar` | 0.2 ↔ 1.0 over **200 ms**, off by default |

So the reference's answer to *"how is depth drawn?"* is **"it is not"** — it is **borders, fixed
chrome heights, a focus colour flip, and nesting**. My earlier question reduced that whole language
to an optional blur. That is the specific ignorance H-12.3 named, and this is the corrected object.

**The question, with the object now whole.** Which language is this shell's?

* **(a) Reference-faithful flat** — 1 px borders, object-name stylesheet, focus flips a colour;
  depth is **nesting only**; no shadow, no blur. This is the reference's own default, and choosing
  it still needs either an MD1 rule or a declared project extension to be admissible here.
* **(b) The MD1 elevation ladder** — dp tiers on **surfaces**, using MD1's own elevations for
  snackbar / FAB / app bar. It rules the *ladder* and **says nothing about blur**, and — the point
  the full read makes unavoidable — **the pane tree has no MD1 equivalent**, so "is a pane a
  surface?" is a **project extension** that must be declared and counted, not read off MD1.
* **(c) The landing's hard displaced solid** — `6px 8px 0`, no blur: one offset quad per panel,
  already implemented twice in `alacripose-landing`. Cheap in a shader, and it is a *shadow*, which
  is the thing the reference deliberately does not draw.

**And the two sub-questions the reading makes unavoidable.**

1. **Per-window or per-pane?** In the reference only the **window** is raised; panes are flat and
   separated by handles. (b) cannot answer this alone.
2. **Which radius rule?** MD1's **single 2 dp** radius — already computed as a rounded-rectangle
   distance field in C-37 — versus the reference's ad-hoc **2/3/4/6/7/12 px**.

**What the evidence still does not settle**, and so is the user's to rule: MD1 rules the ladder and
is silent on blur; the reference's default draws none; the landing's language is a third thing.
A decision here is a **project extension** unless it can cite an MD1 rule by name.

### H-13.1 — What "depth" is in this shell — **answered by derivation, not by a pick**

**Answer (user's words).**

> requires understanding shploingus-clean's design language derived from all families as
> specified in it's ledger

**What it establishes.** The three options offered were a **false choice**. The depth language is
already declared in this tree and derived from the families, exactly as the answer says — MD1
supplies the **form** of the elevation rule and **no magnitude**; the magnitude and the ladder
length are this project's **declared extensions**; the landing and `portal_simulator` are
references for *why* a choice was made, never rulers (C-48). Measured in
`crates/voxel-ui/src/lib.rs` and recorded in C-35 / C-37:

* `ELEV_STEP = 0.05`, `Bucket::ProjectExtension` — *"elevation is a brightness tint of 5 % toward
  white per level"*;
* `MAX_ELEVATION = 4.0`, `Bucket::ProjectExtension` — **five levels, 0..4**, and *"the record
  states no ladder length"*;
* `RADIUS_DP = 2.0`, `Bucket::Md1Rule` — *"MD1: corner radius is 2 dp"*;
* and the code's own words: *"**MD1 elevation is a brightness tint: a raised surface is lighter,
  not shadowed.** `level` above `max_elevation` is **refused rather than clamped**, because a
  clamped elevation is a surface that lies about how high it is."*

So the shell's depth is **raised = lighter**, `ELEV_STEP × level`, levels **0..4**, refused above —
with **no shadow and no blur anywhere in the record**, and with `appearance()` carrying the same
tint as the interaction vocabulary (`Hover` 1 step, `Pressed` 2 steps). **Neither**
`portal_simulator`'s opt-in blurred drop shadow **nor** the landing's `6px 8px 0` is admissible as a
**ruled** value: both describe a *shadow*, and the declared form is a *tint*; adopting either would
also add a **sixth** `ProjectExtension` to a list that is counted **by name** (five today). Derived
in full at **C-55 §55.1**.

**What I got wrong, recorded as the pattern to avoid.** I asked the user to pick among three
references a question the repository had already answered, and the answer names the reason: the
design language is **this tree's**, derived from the families *in its ledger*. Under C-48/C-49 the
outside trees can say *why* a number was chosen; they cannot replace the record's own derivation.
The next question of this shape is to be **answered from the record first** and put to the user only
if the record is genuinely silent.

### H-13.2 — Pane depth — **answered: defer to the arrangement package**

**Answer (selected).** *"Defer to the arrangement package."*

In the reference only the **window** is raised; panes are flat, separated by a 4 px handle and a
1 px border. Here the ladder exists (0..4) but **nothing has been assigned a level**, and H-11.5 /
H-12.2 already make the **shell's own arrangement package** the first package loaded. So *"which
surface sits at which level"* is a **declaration in that package** rather than a constant — and a
declaration above `MAX_ELEVATION` is refused rather than clamped. Recorded at **C-55 §55.2**.

**Round state after this answer.** `continue` is in force (H-12.4). The depth question is **closed
by derivation** and the remaining owed entries are named in **C-55 §55.4**; the order is the record
spec, then the action vocabulary, then the notification queue, then `TODO.md` §7's reorder.

---

## Round 14 — Laya's scope, 2026-09-30

### H-14.1 — What is Laya, in this tree's terms?

**The question.** Raised by the user against the gate's own lines, which answered it the narrow way:

```
oracle-kind      structural-deterministic
oracle-is-laya   no
...
model-oracle     not-evaluated
```

**Answer (user's words).**

> the older agent made laya not the oracle, but laya is the entire decision layer across the entire
> framwork includin the game's systems, and the compiler itself - so it should be using the full
> axis swuite, all of the features, making the game compile it's recipe as a package

**This is a scope correction of the same kind as H-10.5's seasons**, and it names the mechanism of
its own loss: an agent read *"the oracle"* as the whole of Laya, built one trait for it, and the
record then measured the tree against that narrower word.

**What it changes — one seam, and the oracle becomes one implementation of it.**

1. **Laya is the decision layer.** `oracle::Oracle` is *an instrument* that implements the seam,
   not the seam. `crates/voxel-package/src/decision.rs` is the seam: `Subject` is the closed
   vocabulary of what can be decided, `Decider` is the trait every instrument implements, and
   `Assessment` carries the answer, the instrument that gave it and the reason when it is not
   `Active`.
2. **The full axis suite, all of the features, and the recipe are subjects of it.** The five axes
   of `census::Axis` across twelve crates (60), the nineteen feature rows, and the recipe — which is
   decided by **compiling it into a package and loading that package back**, the whole codec — are
   **80 subjects decided in one pass** (H-10.1's *"one clean execution pass"*), and the gate prints
   that pass as its own section.
3. **One vocabulary, reused.** The answer is `features::Establishment` — the same four states the
   feature table and the crate census already speak, for the reason `census.rs` already gives:
   *"a second vocabulary for 'we did not look' is how a gate comes to read as greener than it is."*
   `Unchecked` means no instrument ran and is **not** a pass; `Refused` carries the reason.
4. **The seam does not re-decide.** The deterministic in-project layer answers by **calling**
   `census::census`, `features::evaluate` and `recipe::compile`/`load`, so the seam and the surfaces
   cannot disagree — there is one implementation behind both, and the equality is a test and a
   build-script assertion rather than a hope.
5. **"The game compiling its recipe as a package" belongs to this ruling and is not done.** Today
   `build.rs` compiles `packages/alpha-shell.shspec` and `main.rs` `include_bytes!`s the result, so
   the game receives a package it did not compile — filed as `BLD-0022`.

**What does not change.** L-4's rule stands: *Laya proposes; the ledger records* — a verdict is
never authority on its own, the model instrument is still calibrated and quotable only when it beats
the majority class, and the model oracle is **not evaluated**, which this tree states rather than
implies (`model-oracle not-evaluated`). The deterministic layer reads the tree; it does not decide
what the tree should say.

### H-14.2 — How do the Rust system and Laya relate?

**The question.** Put to the user from H-14.1's consequence, with three shapes offered: the model
out of process, a Rust judge in-project only, or both with the Rust judge deciding.

**Answer (user's words).**

> it's technically supposed to be one in the same? a two pass parser that runs both the rust system
> and the laya rust model through it so everything in the project actually has a visible grading
> according to axis in the ui

and, on the reading that had been offered as the *smallest* cut:

> again you said simpler. I mean the rust suite runs through laya

**What it changes — one seam, two passes, and the second pass is the same kind of thing as the
first.**

1. **They are one and the same instrument, run twice.** Not an either/or: the system pass and the
   Laya pass go through the same `Decider` seam over the same `Subject` vocabulary, so *"the rust
   suite runs through laya"* — the suite's own results are a subject the model layer grades.
2. **`laya` here is a Rust model, in-project.** This tree adopts D97's *host* (Rust, in the
   project, behind the seam) and keeps L-4's *role* (a typed-decision layer, never authority).
   Out-of-process Python stays out (D97's own rule: Python was not banned for being Python, it was
   banned because a run in a foreign runtime cannot have a representation in the client).
3. **Every subject, calibration-guarded.** The model may decide any of the eighty subjects — a
   feature's state, an axis cell, the recipe — and the quotability rule travels with each: a run
   that has not been calibrated *in the same pass* is refused, and a `not_belongs` reading is
   refused unless a human's reason is recorded in the same act.
4. **A disagreement is preserved, never synthesized.** The two passes are reported side by side and
every subject they answer differently is named — hyperskill's own rule (*"a disagreement is
   preserved and adjudicated rather than silently synthesized"*) and the reason the two passes are
   worth running at all.
5. **The grading is visible per axis in the UI.** *"so everything in the project actually has a
   visible grading according to axis in the ui"* — the pass is not a report for a terminal; the
   client is where it is read (H-7.13's rule, and the console window H-7.17 rules).

**A partial inversion is refused by this answer**, and that is recorded because it was offered: the
question put "invert the feature rows only" as the *smallest* cut, and the correction was that the
rule is not *"invert a little"* but *"the suite runs through the layer"*. The inversion is of **all
five axes and all nineteen rows**, and the readers end up reporting observations while the layer
decides what they mean.

---

## Invocation record — hyperskill, high agency mode, 2026-09-30 (round 15)

**The user's instruction.** *"load hyperskill in high agency mode"*, then: *"Your first ticket is to
get the shell running in the client"*.

**Protocol state.** The `hyperskill` skill was loaded fresh this invocation. **`high agency mode` is
explicit, logged, scoped to this invocation and non-inheriting**; it does not answer material
project decisions on the user's behalf and does not remove the hard safety, privacy, evidence,
retirement, destructive-action or current-authority boundaries.

**Lane and budget.** One coordinator lane, no delegated children. Material taxonomy this round:
**scope** (what the bar carries), **dependencies** (whether `recipe/2` could declare it),
**evidence** (the design doctrine, which was read rather than recalled) and **unresolved ambiguity**
(the meaning of "tab", which the user asked back).

**What was read before asking.** `README.md`; `docs/HYPERGRILLING.md`; `docs/LEDGER.md` C-65.4 (the
window manager, read); `docs/TODO.md` §7; `docs/STATUS.md`; `docs/HANDOFF.md`; `docs/UI-SPEC.md`;
`docs/portal_simulator.py`'s four context menus; the dirty branch's `ledger/shploingus-slice1.jsonl`
(event-0004, event-0011, event-0013, event-0018); `docs/design_reference/UNIFIED_DESIGN.md` (752
lines), `ICON_STANDARD.md` (249), the dirty `docs/design/generated/conformance-census.md`, and the
dirty `crates/voxel-client/src/shell/{tabs.rs,tokens.rs}`.

### H-15.1 — What is a bar, and what is a tab?

**The user's question, put back to the agent:** *"what is a tab and what is a bar because shploingus
dirty had a big horizontal bar and desktop portal was planned to have tabs"*.

**Answer (from the record, not from taste).** *A **bar** is the shell's horizontal strip at the top
edge, drawn over a still-running world; a **tab** is one entry on it that opens one floating window,
and a slashed name is one window with two panes — the panes belong to the arrangement, not the bar.*

**Where that comes from, rather than from taste.** The **bar** is the shell's own horizontal strip
at the top edge — `event-0004`: *"Escape pulls a horizontal tab strip down across the top edge of the
screen"*, H-7.15: *"Shell is the upper tab bar"* — drawn over a **still-running** world
(`event-0004`: *"the game CONTINUES behind it"*). A **tab** is one *entry* on that bar, with a stable
id, that opens **one window**: same event, *"Clicking a tab opens it as a floating window"*;
`event-0032` records a click on the strip closing a window as *"correct behaviour for a tab strip"*.
A tab does **not contain** the window, and a slashed name is **one window with two panes** — the dirty
`shell/tabs.rs`: *"a name with a slash is one window with two panes - not two windows"*; the panes are
the arrangement's (§3), not the bar's.

**The simulator's tabs are a different object**, and this is why the word is confusing.
`desktop_portal.py` is byte-identical to `portal_simulator.py` (already recorded), and its planned
tabs are `QTabWidget` pages **inside one application window**: the bizdb plan lists *"Tab 1: Projects
Matrix / Tab 2: Master Tasks Ledger / Tab 3: Revenue … / Tab 4: Persistent Memory"*
(`docs/design_reference/../business_autonomy_architecture.md:56-60`). Those switch views within a
window; they do not open windows and there is no strip. **Both readings are kept**: simulator tab = a
page inside a window; shploingus tab = a bar entry that opens a window. This tree's phrase is the
second.

### H-15.2 — The tab set — **answered**

**Answer (selected).** **Six tabs:** `inventory` (Inventory / crafting), `trees` (Working / crafting
trees), `mods`, `docs`, `settings`, `map-plan`. `governance` and `tools` leave the bar and are
opened by the **console**.

**What it changes.** The bar carries six entries, not UI-SPEC §5's eight. §5's table and the package's
`tab` rows are corrected to the six; `governance`/`tools` remain declared windows with no bar entry.
The declaration mechanism itself was **not** the objection — the user's answer to the format question
was *"I just don't like the changes to what tabs are available is all"* — so membership stays a
declared list rather than being inferred from `lane` or from declaration order.

### H-15.3 — A flat bar of every window is refused; tabs sort the windows — **answered**

**Answer (user's words).**

> yes but it will look dumb and cluttered and I don't even know what streamer is? portal simulator had
> context menus that could show multiple choices of content so tabs should have a dereviation of that
> to sort the windows into the tabs

**What it changes.** The bar does not carry one entry per window (the simulator's own taskbar does,
`:3685-3688`, and it is the clutter the answer rejects). A **tab is a group**, and the windows filed
under it are offered by a **menu on the tab** — a derivation of the simulator's four menus, all read:

| mechanism | where | what it offers |
|---|---|---|
| Start menu | `show_start`, `:3714-3718` | every app in one menu |
| Slot grid | `PanePlaceholder`, `:2601-2614` | the ten choices a waiting slot can take |
| Taskbar button menu | `:139-186` | per-window Open / Properties / Delete / Refresh / Personalize |
| Task-viewer menu | `:863-873` | per-window or per-pane Focus / Minimize / Maximize / Undock / Close |

The offered window carries the task-viewer actions; the tab itself carries the start-menu shape.

### H-15.4 — The filing — **answered, and it corrected an omission**

**Answer (the user's correction, verbatim).**

> you obliterated stuff from the inventory tab

> — and on being asked what: the crafting pane, the Popit tool surface and the recipe trees — all of
> them.

**What it changes.** The `inventory` tab is not one bare window. It holds the **dual pane**
(`event-0004`: *"player inventory/crafting **dual pane**"*; the dirty `TabId::InventoryCrafting`
returns `inventory` + `crafting` as two panes of one window), the **Popit contextual tool surface**
(H-7.25's world shaper wand, H-7.26's *"popit is also used as a design element for the general
surface design"*), and the **recipe trees**. The earlier filing table is corrected accordingly and
is not restated as settled.

### H-15.5 — The streamer window — **answered**

**Answer (user's words).** *"seperate window like console it was supposed to be like the log window"*

**What it changes.** `streamer` is a **standalone window**, a peer of the console and the log — not a
filing under a tab. Its subject (for the record, since the user asked) is the world-streaming engine:
mesh bytes against the ceiling, worker scratch, chunks in flight, chunks refused for budget,
evicted/failed. Pin status for it is therefore a HUD-customizer question, and the customizer does not
exist yet — H-7.24's pinning is the only way a window survives the shell being hidden today.

### H-15.6 — The design lineage set (H-7.29) — **answered**

**Answer (user's words).**

> everything but specifically bringing the old icon substrate shit in cause it was garbage.
> shploingus dirty used it's own icons in the shell

**What it changes — one ruling and one refusal.**

1. **All six lineages bind**: Samsung TouchWiz, Material Design 1, Apple HIG (iOS 6 era),
   Impeccable / DESIGN.md-first, Popit, Advanced Warfare — at equal standing, with §1.0's division
   of *aspect* ownership and *role* ownership (`UNIFIED_DESIGN.md`). H-7.29 is therefore answered.
2. **The old icon substrate is refused** — not the icon *doctrine* but the substrate that came with
   it: the `materialicons` marks traced at 96 px, the seven-family material matrix with its chroma
   ceilings, the corpus ladder over 137 MD1 / 125 TouchWiz / 97 iOS 6 files. The user's words are
   *"it was garbage"*.
3. **This tree draws its own icons**, as the dirty shell did. Measured in
   `crates/voxel-client/src/shell/tokens.rs`: `CHROME_HEIGHT = 24.0`, `CLOSE_TARGET_MIN = 24.0`,
   `CLOSE_GLYPH = 24.0` (*"MD1's system icon is 24dp"*), and the rule at `:231` — *"the hit region,
   the drawn glyph and the declared lane are **one** rectangle"* — over MD1's 8 dp layout grid and
   4 dp icon grid (`:41`). So shell chrome is **glyph work against a declared token grid**, and the
   clean tree's token layer — not an icon asset set — is what has to grow.

**Also recorded: the design spec the clean tree had been missing.** `docs/UI-SPEC.md` is the
*outline*; the doctrine is `docs/design_reference/UNIFIED_DESIGN.md` (six lineages, colour by
mechanism with ceilings, tool-surface rules §5.7, SpringBoard §12, Messages §13, Popit §14, Advanced
Warfare §15, the §17 crosswalk) plus `ICON_STANDARD.md` read for its doctrine and not its substrate.
The clean tree carries a thin slice of this (`voxel-ui`: dark palette roles, ink ladder, 4.5:1,
2 dp radius, tint-elevation, 48/8 dp) and UI-SPEC §8.2 lists four open rows. Carrying the doctrine in
— with every value re-derived here and bucketed — is the work this answer authorises.

---
### H-16.1 — Which named theme lineage does the shell's theme derive from?

**The question.** `UNIFIED_DESIGN.md` §1.0 and H-15.6 settle **ownership** — MD1 owns the metrics
and the icon's silhouette, TouchWiz owns the surface and the colour intent, iOS 6 owns the accent
piece, Impeccable owns how things are made, Popit the contextual tool surface, Advanced Warfare
legibility under load — and all six bind at equal standing. **What none of them answers is what the
window manager's chrome is *made of*, as opposed to who owns which part of it.** A division of
labour across six sources is not a coherent look, and the shell has to be one.

**Answer (user's words).**

> redmond (classic windows 95/98/2000) bliss (xp) longhorn, vista, aero, modern, material (true)
> ios derived and touchwiz derived are the schemes for the way the window manager will look, so aero
> will have glass, classic will be thick pixelated borders and dithered glyphs, modern is modern
> windows 8/10 (preferably closer to 8) ios derived is the ios 6 springboard version (tab bar being
> a dock) touchwiz being touchwiz. then the actual color themes come in over that so users can have
> their windows and tabs in a scheme+theme

**What it changes — the answer is two axes, not one list.**

1. **A *scheme* and a *theme* are separate, orthogonal axes, and the user composes them.** The nine
   schemes are the **chrome language**: `redmond` (Windows 95/98/2000), `bliss` (XP), `longhorn`,
   `vista`, `aero`, `modern` (Windows 8/10, **preferably closer to 8**), `material` (true Material
   Design), `ios` (the **iOS 6 SpringBoard** version), `touchwiz`. The **colour theme** comes in
   **over** that. So *windows and tabs in a scheme + theme* is the unit the user actually picks,
   and neither axis alone is the product.
2. **A scheme decides form, and the answers are specific about form, not about mood.** *Aero has
   glass.* *Classic has thick pixelated borders and dithered glyphs* — which is a statement about
   **border geometry and glyph rasterisation**, both of which are numbers a renderer needs.
   *Modern is Windows 8/10, closer to 8.* *iOS is the iOS 6 SpringBoard version, and the tab bar is
   a dock* — a structural claim about the **bar itself**, not a colour on it. *TouchWiz is
   TouchWiz.*
3. **The tab bar is therefore not one widget.** Under `ios` it is a **dock**; under `aero` the
   chrome is glass; under `redmond` the borders are thick and pixelated. One bar implementation
   cannot be correct for all nine, so the bar's **form is scheme-owned**.
4. **This supersedes a live ruling in `voxel-ui`.** Its test `the_theme_is_dark_and_there_is_no_second_one`
   held that there is one dark theme. **The theme axis now explicitly admits many**, and that test's
   claim is narrower than the record: what is settled is that the *shipped* theme is dark, not that
   a second theme is forbidden. It is to be reworded, not deleted — the reasoning behind it stays.
5. **The debug lane's colour is theme-owned, and the debt is now scoped per theme.** The earlier
   blockage stands exactly as traced: neither `voxel-ui`'s nine roles nor the reference `Theme`
   record's seventeen rows declares a **lane / debug / annotation** row, so the lanes' hue has no
   lawful value *today* — and the row each theme owes is now a **per-theme** row rather than a
   one-off for this tree.
6. **The colour mechanisms are not settled by this answer, and are not assumed here.** `voxel-ui`'s
   seven families (`metal`, `glass`, `ceramic`, `polymer`, `paper`, `road`, `enamel`) with their OKLCH
   chroma ceilings read as a **theme-side** mechanism table, but "aero will have glass" is a
   **scheme-side** word. Whether a scheme fixes its mechanism — and therefore its chroma ceiling —
   while a theme supplies the values inside it, or whether a theme chooses freely and a scheme only
   fixes form, is **not stated by this answer and is not decided here.** It is the next question,
   because the palette cannot be built on an unstated split.

**Carried in immediately, independent of (6).** The scheme axis is *form*, and form is measurable:
border geometry and corner radius, glyph rasterisation (dithered versus hinted), bar height and
entry width, title-bar height, and whether the bar is a strip or a dock. Those are the numbers the
shell currently holds as single constants, and under this answer they become **scheme-owned rows**.

---

## Invocation record — hyperskill, high agency mode, 2026-09-30 (round 16)

**The user's instruction.** *"load hyperskill in high agency mode"* — carried forward from the ticket,
then: *"so in ala-cities there was a past ledger/hypergrill about how the soil and materials move
around"*.

**Protocol state.** The `hyperskill` skill was loaded fresh this invocation (toolchain digest
`sha256:f1fb8e4c…`, context budget `5120/15140`). **`high agency mode` is explicit, logged, scoped to
this invocation and non-inheriting.** It did not decide anything material: both rulings below are
**the user's own words**, taken up from a put-back question. Per the protocol a challenge would go to
a fresh native reviewer before the user is asked; no challenge was raised, so none was needed.

**Lane and budget.** One coordinator lane, no delegated children. Material taxonomy this round:
**evidence** (whether the ala-cities record actually says what the summary claimed — it did not, and
two of my carried claims were corrections), **authority** (ala-cities is reference, not authority,
per C-48), **unresolved ambiguity** (whether substrate's rule and the tick ladder conflict).

**What was read, directly and not through a synthesis.** `docs/design/world-generation-grilling-map.md`
(the distilled map, which says on its face it is *"subordinate, reviewable synthesis; not product
authority"*); `docs/GRILLING-C9.md:1840–1899` (Q166–Q176, the founding-party round), `:1748–1804` (the
`worn:<family>` account and `verify.exe`); `docs/GRILLING-C11.md:5490–5581` (Q705, Q706, Q711), `:5840–5867`
(Q739, Q740); `docs/design/decisions.json` (Q696/Q728 cut/fill/borrow/spoil, Q684, Q691). Then, in this
tree: `crates/voxel-shapers/src/{stages,spec,substrate,climate,lib}.rs`,
`crates/voxel-model/src/conservation.rs`, and the pub-item inventory of `erosion.rs` and
`earthworks.rs`.

### H-16.2 — My carried summary of the ala-cities record was wrong in two places, corrected here

**Correction 1 — I attributed a season ruling to the wrong source, again.** The carried summary
treated ala-cities `gov::season` as adjacent evidence about *game* seasons. It is not: it is a
**governance** period, and the C11 record contains a **different and better** answer than the one I
had. C11's **Q711 "accelerated declared ratios"** says: *"Real-world ratios or evidence are recorded
where available; game-facing rates are declared and measured rather than silently literal."* That is
the rule this tree needs for the ladder's numbers, and it is *not* the rule I had been citing.

**Correction 2 — the erosion cadence does not exist in ala-cities, so there is no conflict to settle.**
I carried "ala-cities says erosion runs at ×20." It does not say that. What it says is **Q684**
(each dynamic process *has* an explicit cadence — a requirement to declare, never a number), **Q739**
(erosion at chunk/voxel scale), and **Q711** (the ratio is recorded evidence or a declared measured
number). The ladder is **H-8.3's**, and H-8.3 is this project's authority. So the substrate conflict I
raised last round **was partly an artifact of my own bad citation**, and I flag that rather than let it
stand as a settled external constraint.

### H-16.3 — The substrate rule and the tick ladder are two subjects, not one contradiction

**The record that settles the shape.** ala-cities never had one substrate; it had **a baseline and a
transport**, and it kept them apart on purpose:

| | on the record | tick-dependent? |
|---|---|---|
| **Baseline** | derived, immutable, re-derived; reconciles against every delta (Q667, Q695, Q733) | **no** — this is `substrate.rs:20`, and it is **right** |
| **Transport** | typed deltas with lineage: cut/fill/borrow/spoil (Q696, Q728) | **yes** — declared cadence, ×20 on H-8.3's ladder |

Q168 (`GRILLING-C9.md:1863`) is the load-bearing sentence and it is about *movement*, not about soil
depth: a dirt road's earth comes from **between** a surface scrape, a borrow pit, and the tile's own
ground, and *"the material it moves **has to go somewhere**."*

**What the conservation record adds, and it is the part I had not read.** `GRILLING-C9.md:1758` — the
`worn:<family>` account — gives the *reason* the destination rule exists, and the reason is not
tidiness:

> a city of structures that have never been repaired balances to **exactly zero** instead of reporting
> its own ageing as material the ground never gave up — the exact class of false finding `verify.exe`
> was corrected for once already.

And Q114 (`:1098`): *"Decay cannot magically disappear. Rotted or respired mass goes to a
**destination account**, never to nothing."* Q174 does the same for fire, into an `atmosphere:`
account. **So "where did it go" is a first-class, audited account in that design — which is why this
tree's `voxel_model::conservation::Ledger` already forces it.**

**The consequence for this tree, which is a ruling and not a refactor.** `substrate.rs:20` is
**reworded, not deleted**: the rule becomes *"the **baseline** profile does not depend on how many
ticks the world ran; **transport** does, on its declared period."* The baseline stays a closed form
and stays tick-free — that is what protects replay (F1). Transport arrives as a **sibling stage** that
emits **deltas and never mutates the baseline**, so the soil at an address is `baseline + Σ deltas`,
which is still pure in `(tick, address)` and is exactly H-8.3's *"the deferred tick is the **schedule**,
never state."*

### H-16.4 — Both questions put back, and answered by the user

1. **Naming / placement.** *Does the transport half become a sixteenth `Stage` beside the existing
   `Substrate`, or a role inside the substrate module?* The user: **"yes sure- probably your best
   assumption"** — recorded as **a sixteenth stage**, which is also the cheap choice, because
   `STAGE_COUNT`/`ALL` are hand-written precisely so that a forgotten stage is a *compile error*, and
   the 15→16 bump is caught by `every_stage_in_all_has_a_dependency_row`.
2. **Which is the rule and which the exception.** *Reword `substrate.rs:20`, or leave it flat?* The
   user: **"Yes same maybe probably"** — recorded as **reword**, keeping the replay reasoning and
   widening only its scope.

**Carried in, not decided here.** `stage_cadences`'s row in `crates/voxel-package/src/features.rs:242`
still names `crates/voxel-work/src/budget.rs` as its witness, and that file is a **byte** budget
(`Budget::new(ceiling)`, `charge`, `release`, `fill`, `fits`) with no clock and no cadence. A green
record row standing over a byte budget is the same false finding the `worn:` paragraph describes, so
it is to be **repointed at the ladder** once the ladder exists — and left red rather than green if it
does not. Also still open and *not* settled by anything above: whether a stage's unit of work is a
chunk, a region, or a column (H-8.3 fixes the periods, not the granularity), and Q691's requirement
that lazy/aggregate updates over inactive regions still **conserve and replay** — that is the guard on
the bounded-chunk-set half of the tick, and it is not yet satisfied by anything in this tree.

---

## Invocation record — hyperskill, high agency mode, 2026-09-30 (round 17)

**The user's instruction.** *"load hyperskill in high agency mode make sure you cross reference
what you're building against the ledger and get going"*, after *"check in the dirty branch as well"*.

**Protocol state.** The `hyperskill` skill was loaded fresh this invocation (base skill digest
`sha256:f1fb8e4c…`, context budget `5120/15140`). **`high agency mode` is explicit, logged, scoped
to this invocation and non-inheriting.** It removes no safety, privacy, evidence, retirement,
destructive-action or current-authority boundary, and it decided nothing material here: the user's
own instruction to cross-reference the ledger is what set the method below.

**Lane and budget.** One coordinator lane, no delegated children. Material taxonomy: **evidence**
(the missing pixels had to be located before anything was written), **authority** (the dirty branch
is reference under C-48, not authority), **scope** (guard first, fix second), **publication**
(C-88 gains a new numbered section and the user's observation becomes the reported evidence).

**Cross-referenced against the ledger, as instructed.** `docs/LEDGER.md` C-17 (line 119) is the
authority for what the mesher claims: *"one greedy pass over a 128³ raster, with no LOD switch"*,
the 34,816 → 6 merge factor, and — the load-bearing sentence — *"greedy rectangles of different
sizes produce **T-junctions**, which can show as hairline cracks at glancing angles. The fix is
vertex stitching; it is *not* in this mesh."* C-19 (line 137) fixes the raster pad at a whole
**voxel** (4 subvoxels) with its arithmetic. C-9 and `crates/voxel-mesh/src/mesh.rs`'s own module
doc both state that meshing and drawing are **one pass** so the two cannot disagree at a boundary.

### H-17.1 — The user's memory was right and my correction of it was wrong

I told the user the missing-pixels fix was not in ala-cities and implied the memory was mistaken.
**The user replied *"check in the dirty branch as well"*, and it was there.**
`/c/Users/ala/Documents/shploingus` (reference only, C-48) contains
`crates/voxel-client/tests/greedy_mesh.rs` with

> `the_greedy_mesh_covers_exactly_the_faces_the_naive_mesher_draws` — *"the load-bearing assertion
> of the whole lane"*

and `crates/voxel-client/src/mesh.rs:443` `build_region_naive`, documented as *"the reference
implementation, and the definition of correct… It walks `visible_faces`, so the two cannot disagree
about which faces exist: what the coverage test checks is that *merging* them loses nothing, which
is the only claim the greedy mesher makes."*

**This clean tree has neither.** `grep -rn "naive"` over `crates/` returns six hits, **all of them
prose comments** in unrelated modules; there is no `build_region_naive` and no `visible_faces`
specification. **The guard against a mesher that drops faces does not exist here.**

**The correction I owe, recorded rather than quietly fixed:** three times this session the answer
was in a tree I had not opened (§ H-16.4 `docs/TODO.md`, §88.3 `docs/APPEARANCE.md`, and now the
dirty branch). C-88.3 already wrote the rule — *"an absence claim gets **read**, not grepped"* — and
**one turn after writing it down I violated the spirit of it again** by searching ala-cities,
finding nothing, and telling the user their memory was wrong. **The rule is not sufficient as
written, because it did not say *which* trees.** It is now: *before claiming absence, enumerate the
trees the record names — this tree, the dirty branch, ala-cities, `shploingus-game` — and say
which were searched.*

### H-17.2 — The approach, and what it deliberately is not

**The port is of the *invariant*, not the implementation.** The dirty tree's types (`VoxelWorld`,
`Direction`, `BlockKind`, `Coord`) do not exist here; this tree's mesher is **subvoxel**-based
(`SUBVOXEL_CHUNK_SIDE = 128`, `MaterialId`, `Face`, a padded `field` raster). So
`visible_faces` and a naive builder are written **against this tree's mesher**, and the dirty tree
is evidence that the shape of the guard is known to work.

**The method is a differential, not a geometry change.** The dirty tree does not stitch vertices
either — it *proves the merge loses nothing* by diffing the greedy mesh against one-quad-per-face
built from the same face set. **So vertex stitching is not this fix**, and the ledger's T-junction
sentence is about a *different* symptom (hairlines across surfaces at glancing angles) than missing
pixels (whole faces absent). Those are two bugs and this entry does not conflate them.

**Written before fixed, on purpose.** The user's own report is the only observation of the missing
pixels, and it has no reproduction in this tree. Writing the guard first means the guard either
**fails and names the exact faces being dropped**, or **passes and leaves the T-junctions as the
documented limitation they already are**. Guessing at a merge-loop fix first would produce a change
with nothing to say whether it helped.
