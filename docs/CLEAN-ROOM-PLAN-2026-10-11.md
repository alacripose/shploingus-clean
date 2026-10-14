# The clean room, worked out — 2026-10-11

> **What this file is.** The checkable work plan the mission of 2026-10-11 asked for: the unified
> systems to be recreated, the **fresh-code boundary** for each (what may be re-derived, what must
> not be copied), the interface each must expose, and the ordered slices for later passes.
>
> **Authority.** It extends [PLAN-2026-10-10-clean-room-and-the-api-layer.md](PLAN-2026-10-10-clean-room-and-the-api-layer.md)
> (the governing plan) and **does not replace it**. Where the two disagree the 2026-10-10 plan wins,
> and the disagreement is recorded below as a *correction*, not a rewrite.
>
> **The rule this plan obeys.** Everything measured is printed with the command that measured it.
> Everything not yet built says *not built*. A unified system with no clean-room code is `absent`,
> never `implied`.

---

## 0. What was read, and where the instructions came from

| Source | Read | What it governs here |
|---|---|---|
| `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` | in full, this pass | the four lanes and the ordering |
| the repository (`Cargo.toml`, `crates/*/src`, `docs/*.md`, `packages/`, `patches/`, `intake/roots.tsv`) | measured, this pass | the inventory and the reconciliation in §1 |
| `https://github.com/github/spec-kit` (README + quickstart + integration + templates docs) | fetched | §4: constitution → specify → plan → tasks |
| `https://github.com/microsoft/agent-forge` (README) | fetched | §4: the `.github/` context-engineering layer |
| P8 https://github.com/gholtzap/lintapple | WIP | NEEDS WORK |
| P9 https://github.com/mikuta0407/win4wine | WIP | NEEDS WORK FOR SHELL |

The 2026-10-10 plan named two upstream tools as the *process* the clean room is built through.
Their published instructions, not memory, say:

* **spec-kit** installs as `uv tool install specify-cli` and initialises with
  `specify init <project>` (or `specify init --here --force --integration <agent>` into an existing
  tree). It shapes work as **`.specify/memory/constitution.md`** plus, per feature,
  **`specs/NNN-name/spec.md` → `plan.md` → `tasks.md`**. The constitution is written **once per
  project**; specify → plan → tasks run **per feature**. In current spec-kit the agent commands are
  `/speckit-constitution`, `/speckit-specify`, `/speckit-plan`, `/speckit-tasks` (older spellings
  `/constitution`, `/specify`… still appear per integration). Plan, tasks and analyze read
  `.specify/memory/constitution.md` **live on every run**.
* **agent-forge** (`npm install -g @agent-forge-copilot/cli`, or `npx @agent-forge-copilot/cli init`)
  is a *Context Engineering Toolkit* that generates the GitHub Copilot customisation layer under
  `.github/` — `agents/`, `prompts/`, `instructions/`, `skills/`, `hooks/`, `workflows/` and
  `copilot-instructions.md` — plus `.vscode/mcp.json`. Its README defines **no** spec-kit
  dependency: it is a complementary layer, not a second spec system.

**How the two are applied here.** spec-kit is the **work structure** and agent-forge is the
**agent context** over it: the constitution states the clean room's principles; `specs/001-…`
states the first unified system's requirements; `plan.md`/`tasks.md` carry the acceptance gates; and
the `.github/` files point Copilot-class agents at exactly those documents so an implementer cannot
work without the constitution in view. Neither tool is allowed to become authority over the record —
the record stays the authority, and every generated document cites it.

---

## 1. Reconciliation: the governing plan against the tree as it stands today

Every row was re-measured this pass. **The 2026-10-10 plan's counts are stale** — the tree grew under
it — so the work plan uses today's numbers and names the drift.

| Kind | 2026-10-10 plan | Measured 2026-10-11 | Command |
|---|---|---|---|
| `.rs` under `crates/` | 239 files / 154 538 lines | **250 files / 169 710 lines** | `find crates -name '*.rs' \| wc -l` · `… \| xargs wc -l` |
| generated code files | 2 files / 7 385 lines | **2 files / 7 397 lines** (`materials/generated.rs` 1 462, `materials/spectra.rs` 5 935) | `wc -l crates/voxel-model/src/materials/{generated,spectra}.rs` |
| `.wgsl` shaders | 6 files / 1 860 lines | **7 files** (`atmosphere`, `display`, `merge`, `panels`, `shading`, `sky`, `wire`) | `find crates -name '*.wgsl'` |
| `docs/*.md` | 26 → 27 files / 25 349 lines | **28 files / 26 538 lines** | `ls docs/*.md \| wc -l` · `wc -l docs/*.md` |
| workspace members | `STATUS.md` said 12 | **16** (`crates/` count in `Cargo.toml`) | `grep -c 'crates/' Cargo.toml` |
| patch records | 39 (`PATCH-001`…`039`) | **39**; the sixteen `hyperskill/*` records named by PATCH-025 are still unnamed here | `ls patches` |
| package files | 526 | **527** | `ls packages \| wc -l` |

**Corrections recorded (not rewritten).**

1. The 2026-10-10 plan's **Lane A step 1** names "all six `.wgsl` files"; there are **seven**.
2. The 2026-10-10 plan orders the clean room **after** lanes A–C ("should not start before they do").
   The 2026-10-11 mission overrides that ordering **for the scaffold only**: the constitution, the
   first spec and a runnable spine are created now, *because they are what makes lanes A–C
   checkable*. No unified system is recreated from an unmeasured record; the record's measurement
   lanes (A–C) continue in parallel. This is a deliberate, recorded divergence, not a silent one.
3. `STATUS.md` §1's "12 workspace members" is stale against today's 16; the clean room does **not**
   touch that file (it is the legacy tree's).

**The gap the governing plan measured, unchanged in kind.** At 2026-10-10, **69 183 lines of code,
shader and prose were named by no record**. That number has not been re-measured since
(`patch::placement` is the instrument); it is **owed** and named here rather than guessed at.

---

## 2. The unified systems to be recreated

A **unified system** here means: a set of behaviours that the game must have, which several crates and
modules currently implement *together*, and which the clean room must re-derive as one fresh whole
behind one interface. The list is closed at **eight**; a ninth is a decision for the record.

> **The decision, taken 2026-10-12 (§16).** The mission of 2026-10-11 names five systems these eight do
> not carry. They are admitted as **U9–U13** in §16 — an **extension on the record**, not an edit of the
> eight entries below. §16.3 records the one correction this pass made *inside* this section: U6/U7/U8's
> `slice:` fields read `6`/`7`/`8` and now read `5`/`6`/`7`, matching §5's rows.

Each entry names:

* **unifies** — what it binds into one system;
* **legacy witness** — where the behaviour currently lives. This is **evidence of the contract only**;
  it is *never* the source the clean room is written from (Constitution P7);
* **fresh boundary** — *may re-derive from* / *must not copy*;
* **interface** — the surface the system must expose to its callers (players' mods and agents go
  through U6/U7, so those two carry the outward doors);
* **slice** — the order in §5.

### U1 — Presentation (appearance + rendering + shell painting)

* **unifies:** the OpenPBR-input derivation, the Source 2 stylisation, the GPU pipeline, the sky and
  the shell's own chrome painting — the thing a player sees.
* **legacy witness:** `crates/voxel-appearance/*`, `crates/voxel-client/src/{renderer,skylight,motion,
  shading,sky,sun,scene,hud,frame,quad,pattern,cloud_cubes,cloud_field}.rs` and the seven `.wgsl`
  files, `crates/voxel-client/src/shell/*`.
* **fresh boundary ✱ may re-derive from:** the user's appearance contract `docs/APPEARANCE.md`
  (OpenPBR + Source 2, 4×4×4 / 256×); `docs/DESIGN-SPEC.md` §2 F13 and §1; the published papers the
  appearance docs cite (OpenPBR Surface 2024; Valve Source 2 shading); the reflectance provenance in
  `LEDGER.md` C-39. **Must not copy:** any file under `crates/voxel-appearance/` or
  `crates/voxel-client/`, shader source included.
* **interface:** `shade(surface, normal, light, view) -> Rgb` (declared types, no implicit state), a
  **measurable** `band_count` over the FMA bar's *≥3 distinct luminance bands* criterion, and a
  declared `Surface`/`Light`/`View` triple. Later, one `Renderer` seam taking meshes + camera →
  pixels. The **band count is the acceptance instrument** and is inherited from the criterion the
  legacy `response`/`npr` tests measure.
* **slice:** **1** — the plan's own step 1, and the only system with zero record coverage for its
  shaders (`placement`). The scaffold implements its first behaviour (this pass).

### U2 — The world model and its vocabulary

* **unifies:** signed coordinates, the 4×4×4 subvoxel lattice and the addressed 256× detail level,
  matter and materials, shapes/profiles, the chain/event ledger, tickets, wire, and the
  civilian/government vocabulary.
* **legacy witness:** `crates/voxel-model/*` (including `axis.rs`, `laya.rs`, `ticket.rs`,
  `materials/`, `civilian/`, `world/`).
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §1–§5, `docs/SHAPERS.md` §2,
  `docs/MODS-MATERIALS.md` (the earth model, never the Minecraft model), the Axis/Laya rules quoted
  in `LEDGER.md` (the seven axes and their prohibitions), and the published shape laws (Zingg,
  Powers, Szymczak, Short–Baygents–Goldstein, Requicha). **Must not copy:** `crates/voxel-model/`
  source, and the two **generated** files (`materials/generated.rs`, `materials/spectra.rs`) — a
  generator is *re-run*, never transcribed.
* **interface:** `Coord`, `Subvoxel`/lattice accessors, `MaterialRegistry` (admit-or-refuse),
  `Refusal`/`RefusalSet` (collect all, name each), `Shaper`+`ShaperSet` (physical vs material across
  the stored/addressed levels), `Profile`, and the one append-only `Chain`.
* **slice:** 2.

### U3 — World generation (the shapers)

* **unifies:** the noise primitives and the fifteen terrain shaper stages plus the process/cadence
  runtime — the world as a pure function of `(seed, tick, declared period)`.
* **legacy witness:** `crates/voxel-noise/*`, `crates/voxel-shapers/*`.
* **fresh boundary ✱ may re-derive from:** `docs/SHAPERS.md` (each shaper names one paper),
  `docs/MISSING-SYSTEMS.md` (the laws and their wiring points, including stream-power and nonlinear
  hillslope diffusion), `docs/DESIGN-SPEC.md` §2 F4–F10, the terrain/materials PDFs under `docs/`.
  **Must not copy:** any `.rs` under `crates/voxel-noise/` or `crates/voxel-shapers/`.
* **interface:** `Declared` (the world's declared stage set), `Stage`, a shaper's
  `acts_at()`-required contract, the `ProcessScheduler` driven by a cadence `TickDriver`, and the
  field plane's `field_at` seam. **Refuse, never default** on every unavailable input.
* **slice:** 3.

### U4 — Meshing, streaming and work

* **unifies:** the sparse voxel octree, greedy meshing, the 256× refinement to triangles, the four
  horizons, the resident set, the worker pool, the budget and the tick driver.
* **legacy witness:** `crates/voxel-mesh/*`, `crates/voxel-stream/*`, `crates/voxel-work/*`.
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5 (one world
  authority; four horizons computed not copied), the published meshing/refinement reasoning in
  `LEDGER.md` C-43 (quads as merge unit, triangles at the displaced level) and C-46 (a measurement of
  the wrong world is worse than none). **Must not copy:** any `.rs` under those three crates.
* **interface:** `Mesh` + `Octree`, `refine`, `WorldSource`, the four typed `Horizon`s, `Resident`,
  `Pool`, `Budget`, `TickDriver` with an idempotent, rewind-refusing `TickReceipt`.
* **slice:** 4.

### U5 — Instrumentation, light and audio

* **unifies:** the one sensor concept, the per-item min/max band, the weighing gate, the gauge wall;
  the light solver and spectra; the audio layer.
* **legacy witness:** `crates/voxel-instrument/*`, `crates/voxel-light/*`, `crates/voxel-audio/*`.
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §3 slots S-03/S-12/S-13 and FC-2.16,
  `docs/light-solver.md`, `docs/lighting-sky-and-moon.md`, and the published spectra tables the
  light crate cites. **Must not copy:** any `.rs` under those three crates.
* **interface:** `Sensor`/`Band`/`GaugeReading`/`GaugeWall` (a reading is a band, never a score),
  `Solver`/`Spectra`, and the audio seam. A quantity that cannot be weighed is refused by name.
* **slice:** 5.

### U6 — The record and the API layer (the gate)

* **unifies:** the content-intake gate, the document record and its compression, the calibrated
  structural judge, the ballot format, the crate-by-axis census, the feature/establishment
  vocabulary, the permission policy matrix, the recipe/content format, the API document, and the
  tool bridge — i.e. everything the plan's **Lane B** funnels the old formats into.
* **legacy witness:** `crates/voxel-package/*` (and `crates/voxel-mcp/*` as its transport).
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §2 F1–F2, `docs/UI-SPEC.md`,
  `docs/SHELL-PORT.md`, `docs/MODS-MATERIALS.md` (an interface declared, not described),
  `.agents/skills/…/SKILL.md` formats, and the record's own decisions on refusals and on `Established`
  vs `bool`. The **sixteen `hyperskill/*` records** are read as *format*, and each old description
  becomes a **declared row with a witness path + digest** (in-place by reference) — never a copied
  body. **Must not copy:** any `.rs` under `crates/voxel-package/`; also must not carry bytes.
* **interface:** `admit/inspect` returning `Established` (not a bool), `Compendium::derive`,
  `StructuralJudge` + `calibrate`, `ballot`, the crate-by-axis census with `Unchecked` never a pass,
  `policy::decide`, `recipe`, `apidoc::render`, and **`bridge::TOOLS`** — one bijective tool table
  over the action vocabulary, with `bridge::call` returning a `Receipt` naming **which checks ran**.
* **slice:** **5** — row 5 of §5, taken with U5 (*written as `6`; corrected in §16.3*). *(Foundational
  for U7/U8, but the plan's step order puts the player-visible systems first; the outward doors open
  here.)*

### U7 — The decision layer (axis · laya · von)

* **unifies:** the seven axes and their prohibitions, the Laya judge's typed `choice`/`score`/`noul`
  questions, the ballot, the calibrated oracle, and the von classifier that answers on CPU.
* **legacy witness:** `crates/voxel-model/src/{axis,laya}.rs`,
  `crates/voxel-package/src/{oracle,ballot}.rs`, `crates/voxel-von/*`.
* **fresh boundary ✱ may re-derive from:** the Axis rules quoted in `LEDGER.md` (rule 51–55, round-5
  bare-token rule, the ladder predicates) and the plan's **Lane C** statement that the join missing
  is *modules → subjects*; the calibration set is the sixteen shared `hyperskill/*` ids at five life
  stages, which are **labelled examples**, not code. **Must not copy:** any `.rs` under those files.
* **interface:** `AxisKind` + `refuse_interchange`/`refuse_reduction`/`refuse_bare`,
  `Question::{Choice,Score,Noul}`, `Verdict::{Belongs,NotBelongs,NoReading}` (three words, no prose),
  `Ballot`, `StructuralJudge`, and `von::answers` whose `Answer` carries `Origin::Oracle` — a model's
  answer is **evidence, not authority**.
* **slice:** **6** — row 6 of §5 (*written as `7`; corrected in §16.3*).

### U8 — The game binary and the shell UI

* **unifies:** the only window in the repository, the mount/shell window manager, the design system,
  input ownership, and the frame loop.
* **legacy witness:** `crates/shploingus/*`, `crates/voxel-ui/*`,
  `crates/voxel-client/src/shell/*`.
* **fresh boundary ✱ may re-derive from:** `docs/SHELL-PORT.md` (the ported machinery and the
  four-bucket rule for every drawn value), `docs/UI-SPEC.md`, `docs/DESIGN-SPEC.md` §3,
  `portal_simulator.py`'s *structure* (reference, never authority), and the design-doctrine rulings in
  `LEDGER.md`. **Must not copy:** `crates/shploingus/`, `crates/voxel-ui/`, `shell/`.
* **interface:** a `Mount`-shaped pane tree driven by a declared `Recipe` (not a hard-coded app
  registry), the two layers' toggle, one pointer-ownership decision read once and applied to both
  grab and visibility, and engine constants sourced from an admitted table rather than literals.
* **status:** **built** (2026-10-12, §27). The interface below is landed in `clean-room/src/shell.rs`
  (**1406** lines): the declared `Recipe` (six windows, and the mount adds none it was not handed), the
  `Mount` pane tree, the shell/HUD **layer toggle** over one window set, the **one** pointer-ownership
  decision read once and passed **by value** into both the drawing and the grab, and the four-bucket
  **admitted constants table**. §5 row 7's check — *a visibly drawn frame; pointer ownership read once* —
  is met by a **100 × 30** drawn frame (digest `79f3885c…`), and the binary's new `shell` mode is the
  interactive form. Owed and named: undock, the splitter-drag resize, the console's five graded levels,
  the toast's final form, and the `gholtzap/lintapple` **P8** gate — which is **U13's**, not this
  slice's, and §27 states why.
* **slice:** **7** — row 7 of §5 (*written as `8`; corrected in §16.3*). It is last because it is the
  only place a person sees the world, and it is the acceptance surface for every system above it.

---

## 3. What "fresh code" means here, exactly

The boundary is one sentence per system and it is checkable:

> **The clean room may read every document, paper, and declared format in this repository. It may
> not copy a line from any implementation file.** The legacy tree is a **behavioural witness**:
> its tests and its measured numbers say what the contract *is*, and its source is not read while
> writing clean-room code.

Mechanically:

1. **No `use` of any `crates/*` path.** The clean room is its own Cargo workspace (`clean-room/` has
   a `[workspace]` table). A `path` dependency on a legacy crate is a **build failure by review** and,
   after U6, a gate check.
2. **Provenance per module.** Every clean-room module's header names the *document* it was derived
   from (the constitution's P1). A module with no named source does not compile-in-review.
3. **Generated code is regenerated, not transcribed.** The two generated legacy files are outputs of
   a declared generator; the clean room re-derives the generator from the sourced data tables.
4. **The witness is cited, never imported.** Where a legacy *format* is the contract (a `.shpkg`
   package, a ballot, a `.shspec` arrangement), the clean room declares a row that cites
   `path + digest` — the plan's **consume-in-place by reference** rule (Lane B).
5. **Nothing is deleted.** The legacy tree is untouched by this program; the clean room sits beside
   it, and the old tree is pruned only as the canon settles (governing plan, Lane B).

---

## 4. How spec-kit and agent-forge are applied

### 4.1 spec-kit — the shape and the commands

| spec-kit piece | Path in this tree | Written when |
|---|---|---|
| Project principles | `clean-room/.specify/memory/constitution.md` | **once** (this pass) |
| Templates | `clean-room/.specify/templates/{spec,plan,tasks}-template.md` | once |
| Feature spec | `clean-room/specs/NNN-<system>/spec.md` | per unified system |
| Feature plan | `clean-room/specs/NNN-<system>/plan.md` | per unified system |
| Task list | `clean-room/specs/NNN-<system>/tasks.md` | per unified system |

The published command sequence, adapted to this tree's agent set:
`/speckit-constitution` once, then per system `/speckit-specify` → `/speckit-plan` →
`/speckit-tasks` → `/speckit-implement`, reading the constitution **live** on every run. The
clean-room directory is initialised against these files with the tool's own command:

```bash
# once, from clean-room/ (the published init; --force only on a throwaway checkout)
uv tool install specify-cli
specify init --here --force --integration copilot
```

**Corrected 2026-10-11 (second pass):** the scaffold above was hand-authored to that exact layout
*first*, and then **the tool was actually run** against this tree — `specify` **v1.1.3** via
`uv tool install specify-cli`, then `specify init --here --force --integration copilot`. The
reconciliation is recorded in §8. Where the tool's generated template differs from ours, **ours
wins** — the constitution is this project's, and the templates encode its acceptance gates; the
init's own run reported *Constitution setup (existing file preserved)*, which is the behaviour that
keeps our P1–P7.

### 4.2 agent-forge — the context layer

agent-forge generates GitHub Copilot's project configuration under `.github/`. Applied here it
produces the files that make the constitution and the active spec **impossible for an implementer
agent to miss**:

| agent-forge artifact | Path | Purpose here |
|---|---|---|
| Project context | `clean-room/.github/copilot-instructions.md` | points every agent at the constitution + active spec; restates the fresh-code boundary |
| File-pattern instruction | `clean-room/.github/instructions/clean-room.instructions.md` | `applyTo` the clean-room sources; P1 provenance header + P7 no-legacy-import enforced at author time |
| Agents / prompts / skills | `.github/agents/rust.agent.md`, `.github/prompts/shploingus-clean-room.prompt.md`, `.github/skills/rust/SKILL.md`, `.github/instructions/rust.instructions.md` | **generated 2026-10-11** by `forge generate` over the real Copilot CLI; see §8 |

The dependency direction is one-way: **spec-kit owns the work; agent-forge points at it.** No
agent-forge file becomes authority over the record.

---

## 5. Ordered slices for subsequent passes

Each slice ends with a **check** that a person can run. A slice not checked is not done.

| # | Slice | System | Acceptance check |
|---|---|---|---|
| **0** | Constitution + first spec + runnable spine *(this pass)* | U1 | from `clean-room/`: `cargo test` runs the band-count behaviour (9 passed); `cargo run` prints the registry and the measured band count, exiting 0 |
| 1 | U1 complete: `shade` over declared `Surface`/`Light`/`View`; the FMA bar measured as a number; the sky's radiance from the atmospheric state; a **visible** client (never `nodraw`) | U1 | the band count and a screenshot; C-40 (an eye is the check that finds the most) — *(2026-10-13, **§36**: **the visible client is built** — a real window, the world's own cells drawn through the contract's one `shade` call, one tick of the plane's own law per frame, the window's own pixels read back and compared at **66 355 200 of 66 355 200**, and the frame written as a P6 screenshot two runs write identically (`docs/baselines/u1-visible-client/`). What this row still owes is its **GPU pass** — a shader checked against `presentation::shade`, T009's other half — a resizable window, the Sun's azimuth and the sky's radiance as a **term** (the client reads the solver's Sun, not the sky). And **the shape of the picture is ruled rather than open**: this row's client draws a **column** world while the record's own coordinates are cubic (`32³` chunks, three axes), and the user's own report is that the **cubic chunks format caused the material flicker in the WGSL path** and is why this branch exists — so the column representation is **refused for cause**, the GPU pass is owed as a shader over **voxel faces** rather than as a shader for this picture, and `PATCH-040` carries the finding and the mechanism its audit must establish)* |
| 2 | U2: coordinates, lattice, materials (sourced reflectance admitted-or-refused), `RefusalSet`, shaper kinds × levels, the chain | U2 | **partially built (2026-10-11, §7)**: identity, coordinates, materials, the shape laws + shaper taxonomy (`ShaperSet`) and the chain/tickets/wire pass; the civilian vocabulary (Axis/Laya) is still owed |
| 3 | U3: noise + the fifteen shapers from `SHAPERS.md`, the cadence runtime | U3 | each shaper named by one paper; one digest for one `(seed, tick)`; everything but the deliberate refusals runs from tick 0 — **built (2026-10-11, §9 and §10)**: the roster, the primitives, the cadence runtime and **thirteen of the fifteen stage bodies**; one digest at 1/2/4/8 workers = `460214b1…2fc6`; the tick-0 tally is **13 of 15** with 2 refusals by name (`caves`, `earthworks`), which is the legacy's `14 of 16` floor read against this fifteen-stage roster |
| 4 | U4: octree + greedy mesh + `refine`; the four horizons; the pool | U4 | merge factor and per-chunk memory re-measured; one digest across 1/2/4/8 workers — **first slice built (2026-10-11, §11)**: the field-derived mesh with its greedy merge, the level ladder `0..=2`, the four typed horizons, the resident set keyed by world truth with exact invalidation, the work ledger counted where the work happens and a budget refused by name; `1024 columns → 144 quads` (merge factor **7.16**), one digest `0e7358e4…4ed8` at **1/2/4/8** workers; the octree, `refine` (the displaced 256× level), a **region's own residency** rather than its one-footprint stitch and the streamed residency policy are owed — *(§39 moved the mesh's unit to the cells' own faces and §40 landed the extent contract and the two-chunk stitch: see §39 and §40)* |
| 5 | U5 + U6: instruments, then the record/gate with `bridge::TOOLS` served over stdio | U5, U6 | an unadmitted package refused by name; `Established::ChecksumOnly.establishes_identity() == false`; a tool call's `Receipt` naming the checks that ran — **U6's first slice built (2026-10-11, §12)**: the closed five-verb table, the canonical request line, the answer whose digest is a pure function of what it describes, the claims re-checked against the owner each describes, the effect as a delta of what the owners hold, and the split that makes a parse fault write nothing while a world refusal is named **and recorded**; one request twice prints one answer digest `25a485a5…` with `1024` then `0` columns, and the same receipt refuses `claim_unverified` against a moved world with both values. **U5 built (2026-10-12, §24)**: `measure.rs`'s one sensor concept with the wall and the overlay as its consumers, a min/max band per item, a weighing gate whose answer is a mass and nothing else, the virtual gauge wall; `light.rs`'s one band table and the closed-form solver (sky, beam, twilight, the Moon as a reflectance, two droplet geometries); `audio.rs`'s onset read — with **a quantity that cannot be weighed refused by name** (`not_weighable`), the closed vocabulary at 65 kinds, and the binary at 457 lines of stdout *(corrected 2026-10-12, §30–§31: the voices then moved U5's own figures — the closed vocabulary 86 → **90**, the suite to **365 / 0**, the binary to **498** stdout lines; the 65 kinds and 457 lines are this row's own, pre-voices build)*; U6's remainder — the document record and its compression, the structural judge, the ballot, the policy matrix, the API document, a pin over a request line, the crate × axis census, `save`/`load` as rows — **is still owed** *(corrected 2026-10-12, §28: `save`/`load` **have** since landed — they are the table's seventh and eighth rows, so it is closed at **eight** verbs — and U9's document form is served through them)* *(corrected again 2026-10-12, §29: the **permission policy matrix** has since landed too as the policy chokepoint — one decision function, an act per row declared from the row itself (`Tool::action`), deny by default made structural, and every denial recorded — so the matrix is off this list and its **own** remainder is named where it moved: the self-describing protocol and the tool-surface narrowing over a mod-supplied tool list. The document record and its compression, the structural judge, the ballot, the recipe format, the API document, the crate × axis census, a pin over a request line and the legacy-vocabulary bijection stand owed)* |
| 6 | U7: axis/laya/von wired — the join **modules → subjects** the plan's Lane C names | U7 | a ballot answered with `Origin::Oracle`; the calibration accuracy beats the majority class — **built (2026-10-12, §25)**: the seven axes (three ladders declared, four owed by name) with their three prohibitions, the coverage table's total pin and anti-fabrication guard, the three-word verdict, the typed questions, the ballot whose floor is declared before the answer exists, the four fail-closed rules each proved to bite by name **and** by a live perturbation restored byte-exact, and the classifier reading the record's **declared head** — `belongs`, origin `A`, leave-one-out **0.5000** against a majority class of **0.3125** (margin **+0.1875**) with the claim-only reading **0.0000** printed beside it, because that is the measurement that moved the question off the claim; owed and named: the score instrument's rungs, the `noul`/`score` calibration sets, a model-backed instrument, U6's remainder and the four U10 rulings plus `mc-ar-mod` |
| 7 | U8: the window, the mount, the design system | U8 | a **visibly** drawn frame; pointer ownership read once — **built (2026-10-12, §27)**: a declared `Recipe` mounted as a pane tree, the shell/HUD layer toggle over one window set, one pointer-ownership decision read once and passed by value, a four-bucket admitted constants table (16 rows), and a **100 × 30** frame whose six panes read the registry, the reference world, its materials, the stage roster, a derived chunk's geometry and the light table; the binary's `shell` mode is the interactive check. Owed: undock, the splitter-drag resize, the console's five graded levels, the toast's final form, and the **P8** `lintapple` gate (U13's) |
| 8 | Lane A/B/C measurement on the clean room itself: `placement` and `patch-coverage` re-run against the new tree | all | lines-named-by-a-record reach the bar the plan sets |
| 9 | U9: saves and loads, and the world's own seed systems | U9 | a written world reads back as the same world: save → load → one `truth_digest`, and a rebuilt world equals a ticked world at 1/2/4/8 workers; a save missing a seed or a ruleset identity is refused by name — **first slice built (2026-10-12, §17)**: the exact length-framed encoding with its trailer, **five** declared per-domain stream seeds carried and checked in both directions, the ruleset identity, the record carried in its own wire form and declared by head and length, six refusals by name, a real file round trip (`465 bytes`), one `truth_digest` for the loaded and the live world (`32b3c1f8…`) and one generation digest at **1/2/4/8** workers (`c331cf55…`). Owed: the **document form** of a save (`path + digest`, where U6's content gate becomes a hard prerequisite), `save`/`load` as rows of U6's table, and a save of a resident set — *(both forms now built: the document form and the two rows landed 2026-10-12, **§28**, so what this row owes is a save of a **resident set**, **compression** and a **versioned migration path**)* |
| 10 | U10: the mods-derived gameplay tree | U10 | every row names its document, its licence verdict and the system that carries it; a refused reference is refused by name; the tree's first row is playable — **first slice built (2026-10-12, §23)**: the tree is **106 entries** (one per reference in `MODS-MAP` §1–§5, in the record's order: mapped 64, open 25, refused 5, unresolved 12) and **43 technology rows** (23 cited by more than one reference, 35 with their own bar owed); U6's own census over the rows reads `177 established / 121 inactive / 0 unchecked / 3 refused — admitted 8 of 43`; `BigGlobe`, `lithostitched`, `Create-The_Factory_Must_Grow`, `MFR` and the re-keyed `archipelago` datapack refuse `licence_refused`, an absent document refuses `source_absent` and a moved one `source_digest_mismatch`, and the head (`chunk-infrastructure`, U4) is admitted |
| 11 | U11: planet shploingus at planet scale | U11 | a scale/extent contract is declared and the streamed residency policy is measured — nothing is resident that was not asked for |
| 12 | U12: civilian organisations for mobs and autonomous players | U12 | an organisation's member acts only through the one tool table, and an act it may not do is refused by name and recorded — *(2026-10-12, §29: the **second half of this check is now measured** rather than awaited — an actor holding nothing is refused `policy_denied` by the one call site and **each denial is recorded** on the one chain, so what this row still awaits is the organisation's own vocabulary, not its enforcement)* *(2026-10-13, §32: **the first slice is built** — `src/being.rs`'s first mob is a real member: the charter is read over the whole closed action vocabulary (13 acts, 7 reached), a granted act is taken and an act the office does not reach (`save_world`) is refused `policy_denied` **and recorded**; the row is no longer half-measured. What it still awaits is unchanged and now measured in place: U2's **T016** vocabulary — the office rank and the certification bands print **owed by name** in `axis.rs`, and this pass took the smallest reading rather than inventing a rung — and U6's self-description)* |
| 13 | U13: the shell completed, and its gholtzap/lintapple gate | U13 | a visibly drawn frame (U1's band count and a screenshot) with every constant admitted, and the **P8** gate passing rather than `WIP` |

**The next slice is 0 → 1**: finish U1's spec through `plan.md`/`tasks.md`, then implement the full
`shade` + sky contract behind the spine this pass creates.

**Corrected 2026-10-12 (the decision pass, §16).** That sentence was written in pass 1 and is now
stale: passes 2–6 ran ahead of it — U2 (§7), U3 (§9–§10), U4 (§11), then U6's gate (§12) and its stdio
transport (§13, repaired in §15) — so **row 1's remainder (U1 complete) and row 5's first half (U5)
are still owed**, and U6's own remainder is owed inside the row it shares with U5. Rows 9–13 above are
the five systems the mission's completeness clause names, admitted in §16. Row 8 is a **standing gate**,
not a blocker in front of rows 9–13: it is re-run after each of them.

**Corrected 2026-10-13 (§36).** Rows 1 and 5 have both moved since that paragraph was written: U5 is built
(§24–§31) and **row 1's visible client is built** (§36), so what row 1 still owes is its **GPU pass**, a
resizable window and the Sun's azimuth — and all three are now **behind `PATCH-040`** rather than ahead of it,
because the client this row just built draws a **column** world while the record's own coordinate law is **cubic**
(`32³` chunks on three axes). The user's ruling is that the cubic chunks format is out **for cause** — it is what
produced the material flicker in WGSL, and it is why this branch exists — so the audit owes the flicker's
**mechanism** and the migration order (the identity, the seam and the client), and no further row-1, column-shaped
work should start first; §36's "Owed, and named" carries the same finding.

---

## 6. Status of this pass, honestly

**Created:** this plan; `clean-room/` (spec-kit structure, constitution, spec/plan/tasks for U1, the
agent-forge context files); and a runnable Rust spine whose first behaviour — the Source 2 four-term
shading and the *≥3 distinct luminance bands* criterion — is re-derived fresh and tested.

**Not done, and named:** U3 is **in progress** (§9–§10: thirteen of fifteen stage bodies, the two
remaining refusals being decisions), U4 is **in progress** (§11: the field-derived mesh, its level
ladder, the four horizons, the resident set and the work ledger — the octree, `refine` and the stream
policy are still owed), U5–U8 are
`absent` *(superseded for U6 — see the correction at the end of this section)*; U1 is `specified + one
behaviour`, not complete; the governing plan's `placement` gap is
un-re-measured; the clean-room gate that forbids a legacy `use` does not exist yet.

**Corrected 2026-10-11 (second pass):** `specify init` **has** now been run (v1.1.3) and so has
agent-forge's generator (`forge generate`, Copilot CLI backend) — the files they own are theirs, and
spec 003 was created through spec-kit's own workflow. §8 records the commands, the versions, the
files produced and what was reconciled.**What the legacy tree got:** nothing but one new document. No file under `crates/` was read while writing clean-room code, and no legacy file was modified.

**Corrected 2026-10-12 (the decision pass).** The status above is superseded where it says U5–U8 are
`absent`: **U6 landed** — the closed tool table, the request/answer/receipt path, the gate (§12) and the
stdio transport (§13), with the two audit defects fixed at their cause (§15). Of U5–U8, **U5, U7 and U8
remain `absent`**; U6's own remainder (the content gate and everything §15 lists as still owed) is owed
inside its slice; and U1 is still `specified + one behaviour`. The mission's five systems are admitted
as U9–U13 in §16.

---

## 7. U2 passes — identity, coordinates, materials, shapes and the chain (2026-10-11)

The second ordered slice, in two passes. **Built and measured** in `clean-room/`:

- **World identity and determinism** — `WorldIdentity { seed, declared period }` plus `Tick`; truth is
  a pure function of `(seed, period, tick)` and is digested. The **anchor** and the **residency
  cache** are separate concerns and are provably invisible to truth (`World::truth_digest`).
- **Coordinates and placement** — signed 32-bit `VoxelPos` (z up), Euclidean chunk mapping
  (so `x = -1` is chunk `-1`), the stored 4×4×4 `SubvoxelIndex`, the addressed 256× `DetailIndex`, the
  corrected 20-level ladder with `RegionKey`, and the **face relations** (`chunks_reached_by` returns
  the containing chunk plus one per boundary face — at most three, never a diagonal).
- **Materials** — one `MaterialRegistry` that admits once and refuses an unadmitted or duplicate id
  by name; **exact rational** densities in g/cm³; reflectances carried **with a source** (C-39), and a
  material with none admits but derives no colour.
- **Refusals** — `Refusal`/`RefusalSet` collecting **every** problem, with named kinds; plus the one
  content identity `Digest` over SHA-256 (the single justified dependency, `sha2`).
- The crate is now a **library plus a thin binary**, so the public API is a real API exercised by both.

**Evidence.** `cargo test` → **31 passed**; `cargo clippy --all-targets -- -D warnings` → clean;
`cargo run` prints the truth/residency/plan digests and the registry digest and exits 0. The measured
invariances: truth with a moved anchor and a populated cache is **identical**; truth at tick 1
**differs**; residency digest is **identical** across insertion orders; the plan digest is
**identical** forwards and reversed; a negative coordinate recomposes exactly; a corner voxel reaches
three chunks. The coordinate round-trip test checked **2 803 221** coordinates exactly.

**Second pass (same day): the shape laws, the shaper taxonomy and the chain** — T013–T015, built and
measured:

- **Shape laws (`src/profile.rs`)** — `Profile` over the Zingg/Powers/Szymczak shapes (flat, cylinder,
  frustum/cone, spheroid, stalagmite `Da` law), each displacing with its own **exact** normal (a flat
  displaces with its plane's normal, never a fallback), a malformed profile refused by name, and the
  silhouette radius measured against the shape.
- **The shaper contract (`src/shaper.rs`)** — `ShaperKind` × `Level` (the two kinds × two levels), a
  `StoredLattice` at the addressed level, `apply`, and `ShaperSet` where a shaper claiming the
  addressed level while declaring nothing is **refused by name**.
- **The chain (`src/chain.rs`)** — the append-only, digest-linked event ledger: an event links to the
  head; an illegal append (rewrite, wrong predecessor) is refused by name; tickets and an **exact**
  wire form that round-trips byte for byte; `Namespace` closes the vocabulary so an empty one cannot
  be constructed.

**Still owed inside U2** (named, not stubbed; `clean-room/specs/002-world/tasks.md` T016): the
civilian/government (Axis/Laya) vocabulary. This pass chose depth on T013–T015 over breadth across
all four.

**What the legacy tree got, again:** nothing. No `crates/` source was read while writing clean-room
code, and no legacy file was modified.

**Repair pass (same day), from the read-only audit:** every reproduced defect was fixed at its
cause, each pinned first by a test inside the crate. `plan_digest` is a sorted **canonical** digest
(a duplicated chunk no longer cancels to the empty plan); `Chain` checks an event's **meaning** in
one place (`check_semantics`), so the writer, the wire reader and `adopt` cannot disagree — a
resolution of an unfiled ticket and a tab-bearing payload are refused everywhere; `Cylinder` refuses
a zero height, as its docstring always said; `chunks_reached_by` refuses at the world edge instead
of returning a chunk whose `origin()` overflows; `Digest::of_parts` frames each part by its length
and `from_hex` accepts only the lower case it documents; and the stalagmite is **one body** again —
the linear silhouette and the law's normals (which disagreed) are replaced by a recorded
**elliptical flank** keeping the law's exact consequences (cap radius, radial base normal, axis cap
normal), because the law's own meridian is an elliptic integral this module refuses to approximate.
U1's "zero external dependencies" SC-004 and U2 §5's `Shape` / `LadderLevel` lines were corrected to
the truth rather than satisfied with invented code. `cargo test` → **52 passed**; clippy clean; the
binary exits 0; the audit probe reports **0 findings**. Still owed: T016 (Axis/Laya) — and the
generator run, which was owed here, is **done** and recorded in §8.

---

## 8. The generator run — spec-kit and agent-forge, run for real (2026-10-11)

The audit that preceded this pass found the process requirement unmet: `clean-room/.specify/**` and
`clean-room/.github/**` were a hand-authored *imitation* of the two generators' layouts (no
`.specify/scripts/**`, no `integration.json`, no `.github/skills/speckit-*`). This section records
what running the real tooling did, command by command, and what was reconciled rather than overwritten.

### 8.1 spec-kit — `specify`

| Step | Exact command | Result |
|---|---|---|
| install | `uv tool install specify-cli` | `specify` **1.1.3** on PATH |
| reconnaissance | `specify init --here --force --integration copilot` into a scratch dir under `%TEMP%` | the output tree was inspected **before** the real run, so the reconciliation was deliberate |
| the real init | `specify init --here --force --integration copilot` in `clean-room/` | created `.specify/{scripts/bash,integrations,memory,templates,workflows}`, `.specify/integration.json` (`version: 1.1.3`, `integration: copilot`), `.specify/init-options.json`, `.specify/feature.json`, and `.github/skills/speckit-{specify,plan,tasks,implement,analyze,checklist,clarify,constitution,converge,taskstoissues}/SKILL.md` |
| what it said about our work | — | **“Constitution setup (existing file preserved)”** — the init did **not** overwrite `constitution.md`; our P1–P7 stand |
| verified untouched | `diff -rq` against a pre-init backup | `src/`, `specs/`, `Cargo.toml`, `README.md`, `constitution.md` unchanged; only `installed_at` timestamps differ from the scratch run |

### 8.2 agent-forge — `forge`

Installed and invoked through npx: `@agent-forge-copilot/cli` **0.3.0**. Its AI paths need the GitHub
Copilot CLI, which was **not** installed, and the failures below are recorded because they are the
honest trail rather than a story:

| Attempt | Exact command | Result |
|---|---|---|
| prerequisite | `forge check` | `✗ Missing required tools.` — `Required: GitHub Copilot CLI: npm install -g @github/copilot (must be a global install)` |
| analyze (interactive) | `npx @agent-forge-copilot/cli analyze` | timed out at 600 s waiting on an interactive model pick; no files written |
| analyze (`--model`) | `… analyze --model claude-sonnet-4.6` | `Error: Model "claude-sonnet-4.6" from --model flag is not available.` — the message is the Copilot CLI's own |
| templates mode | `… init --mode templates --use-cases code-review` | `Failed to install code-review` — **static template generation was removed upstream**: `src/lib/scaffold.ts` states *“Static template generation has been removed — Copilot CLI is required”*, and the published package ships no `templates/` gallery (`package.json` `files: ["dist","README.md","LICENSE"]`; the repository at `655ff34` has no such directory either) |
| the backend, installed locally | `npm install --prefix <temp> --no-save @github/copilot` | **1.0.95** installed into a temp prefix instead of the global install `forge check` asks for; detection succeeded because `copilot --version` is looked up **on PATH**, which this run provided. `copilot -p "Reply with exactly: PROBE_OK"` → `PROBE_OK`, exit 0 |
| the model the account actually has | `copilot --model gpt-5.4 -p …` / `--model claude-sonnet-4.6` | both refused; **`--model auto` accepted** (`AUTO_OK`, exit 0) |
| **the real generation** | `npx @agent-forge-copilot/cli generate "<description of the clean room and U3>" --mode full --model auto` | **exit 0**, 57 s, plan-then-execute pipeline (“Fleet”), **5 files**, ~2 PRU |
| its own validator | `npx @agent-forge-copilot/cli validate` | **exit 0** — *16 file(s) passed*; 20 warnings, all on spec-kit's own `speckit-*` skill descriptions (missing “USE FOR:” / “DO NOT USE FOR:” phrasing), reported as auto-fixable with AI and **left unfixed** |

**Files agent-forge produced** (this is the generator's own output, not an imitation):

| File | What it is |
|---|---|
| `.github/agents/rust.agent.md` | the “Rust Clean-Room Library” agent, scoped to the deterministic voxel-world crate |
| `.github/instructions/rust.instructions.md` | `applyTo` the crate's files; states the refusal rule, the zero-dependency constraint and the spec-first rule |
| `.github/prompts/shploingus-clean-room.prompt.md` | the implement-and-verify prompt, pointed at `specs/` |
| `.github/skills/rust/SKILL.md` | the on-demand skill, with the USE FOR / DO NOT USE FOR phrasing its validator wants |
| `.github/instructions/clean-room.instructions.md` | **merged**: the generator prepended its frontmatter to our hand-authored file |

**There is no marker to look for.** agent-forge 0.3.0's `generate` path writes **no** manifest and
stamps no “Generated by” line into the files it produces; the only project-visible trace is the
files themselves plus the run's log. An audit should therefore **not** infer “the generator was never
run” from the absence of a marker — it should check for the artefacts and for this section. (spec-kit,
by contrast, does leave machine-readable traces: `.specify/integration.json`,
`.specify/integrations/*.manifest.json`, `.specify/workflows/workflow-registry.json`.)

### 8.3 What was reconciled, not overwritten

1. **The constitution is ours.** `specify init` preserved it; agent-forge does not own it. P1–P7 stand
   unchanged.
2. **`clean-room.instructions.md`** kept the generator's frontmatter but the generated
   `applyTo: "**/*"` was **narrowed to `src/**/*.rs`** — the file's rules are about Rust sources, and
   a `**/*` glob would have claimed the specs as well. Its body (is untouched) and its stale
   “hand-authored because the generator has not been run” comment was corrected to the truth.
3. **`copilot-instructions.md`** — written by spec-kit's integration, kept, with its stale header
   corrected and **spec 003 added to its list of authority documents**.
4. **`specs/001-presentation` and `specs/002-world` keep their authored shape and substance.** They
   predate the generator's arrival; rewriting them to the template's headings would have changed
   documents that were honestly authored, and the mission is to reconcile rather than rewrite. What
   is authored stays authored; what the generators own is now theirs.
5. **The executable checks are untouched**: `src/`, `Cargo.toml` and the 52 tests are exactly as the
   repair pass left them.

### 8.4 Spec 003 — created through the workflow, not by hand

The next feature (U3 Shapers) was created with spec-kit's **own** commands, from `clean-room/`:

```bash
.specify/scripts/bash/create-new-feature.sh --dry-run --json --number 3 --short-name "u3-shapers" "U3 world generation: the shapers"
#  → {"BRANCH_NAME":"003-u3-shapers", ...}
.specify/scripts/bash/create-new-feature.sh --number 3 --short-name "u3-shapers" "U3 world generation: the shapers - noise primitives and the fifteen terrain shaper stages plus the process/cadence runtime"
#  → specs/003-u3-shapers/spec.md copied from .specify/templates/spec-template.md; .specify/feature.json written
.specify/scripts/bash/setup-plan.sh     # → specs/003-u3-shapers/plan.md from plan-template.md
.specify/scripts/bash/setup-tasks.sh    # → reports FEATURE_DIR, TASKS_TEMPLATE and the missing docs
```

The artefacts the workflow's skills require were then filled from the **U3 content this plan names**:
`spec.md` (three prioritised user stories, the fifteen-stage roster with one paper each, the two
genuinely absent laws — stream power and nonlinear hillslope diffusion — the two deliberate refusals
`caves` and `earthworks`, FR-001…FR-010, SC-001…SC-007), `plan.md` (Constitution Check P1–P7, real
module layout), `research.md` (D1–D6 decisions with rejected alternatives), `data-model.md`,
`contracts/u3-seams.md`, `quickstart.md`, `checklists/requirements.md`, and `tasks.md` in the
generator's checklist format (T001–T028, six phases, `[US#]` labels, the `14 of 16` floor).

**No U3 code was written.** `src/systems.rs` still records U3 as `absent`, and every box in
`tasks.md` is unchecked.

### 8.5 Still owed, named

- **T016** (Axis/Laya vocabulary) — unchanged from §7.
- **U3 itself** — specified and planned in this pass, **built** in the next (§9) and **finished** in the
  one after (§10).
- **agent-forge's 20 warnings** on spec-kit's skill descriptions — reported, deliberately not
  AI-fixed in this pass (an AI rewrite of another generator's files would blur ownership).
- **The two open scoping questions** recorded in `specs/003-u3-shapers/research.md`: whether the
  stream-power and hillslope-terrain terms are wired to elevation in slice 3 or deferred to slice 4,
  and the exact (exact-arithmetic) tolerance for the region conservation recount.

---

## 9. U3 slice — the shapers, and the first digest (2026-10-11)

**What landed.** `clean-room/src/` gained `noise.rs` (gradient, cellular, fBm, ridged, domain warp,
the splitmix64 hash), `stage.rs` (the closed fifteen-stage roster vocabulary: `Cadence`'s four rungs,
`FieldId`, `Patch`, `Stage`, `Declared` and its validation, `ProcessRequest`, `StageContext`,
`Slopes`), `shapers/{terrain,water,air,life,record}.rs` (the first eleven stage bodies plus the three
owed ones and the record), `process.rs` (the reference declaration, the level scheduler with the real
`workers` knob, the tick driver and the one fold), and `field.rs` (the `field_at` seam). `main.rs`
now prints the U3 evidence and exits non-zero if a bar is unmet.

**The measured numbers** (from `clean-room/`, the four commands at the end of this section) — **as
measured in that pass**: the digests and the tally moved when §10 closed the record bodies and
re-declared the reference world's dials, and §10 carries the current figures.

| Claim | Measurement (printed) |
|---|---|
| SC-001 one digest, 1/2/4/8 workers | `089ceed021af59052f3e9d651cc92d183e7c7271477d6e5831846af0d93108a4` at **all four** worker counts |
| SC-002 a moved tick moves it | tick 1 → `f0cb0822efd2147634cd4957d57b6aaba8d6df3c04217693f65430e6161702cf` (differs) |
| C4 the anchor and the residency cache are not inputs | `the_digest_is_blind_to_the_anchor_and_to_a_populated_cache` passes |
| The tick-0 tally | **11 of 15 ran, 4 refused by name**: `caves`, `delta`, `transition` (`body_owed`), `earthworks` (`not_a_generator`) |
| C2 idempotent + no rewind | tick 3 driven twice is one receipt; tick 2 after tick 3 → `rewind` |
| FR-007 the one seam | `elevation` from `terrain` (64 columns); region-only `discharge` at a lone site → `region_required` |
| SC-005 conservation | the pairwise flux conserves exactly, with the exchange bound at **half** the pair's difference (the operator was later folded into `hillslope_divergence`; §10 measures the sum) |
| Declarations | 15 stages, declaration digest `acb977d9e25075704e2c71e566bb0a45eeb4daa2958d2d14aa23bfcb0d56db51` |
| Gates | `cargo test` **102 passed**; `cargo clippy --all-targets -- -D warnings` clean; `cargo run` exits 0 |

**Two corrections the implementation forced, recorded rather than absorbed:**

1. **The flux bound is half the difference.** Moving `x` from `a` to `b` closes their gap by `2x`, so
the contract's "a flux larger than the difference" admitted exactly the case that inverts the pair.
The operator refused more than half (`u3-seams.md` C3 records it) — and §10 then **removed** the
standalone operator once the hillslope divergence became the measured conservation path, so the bound
lives in the contract and the tests rather than in a second transport law with no consumer.
2. **`ecology` classifies on the climate alone.** Soil was a multiplicative term on the humidity
ratio in the first draft, which made a steep hillside refuse a climate it could classify — a real
defect the reference run exposed (the tally said 10 ran). The province is now Holdridge's geometric
humidity-province ladder, and `substrate`'s soil depth decides *establishment* (a reserved bare
class below a declared threshold). Lieth's capacity and Reynolds' flocking stay **owed**, and
`ECOLOGY_SOURCE` no longer cites a paper whose law is not implemented.

**Still owed from this slice** (named, not implied) — **all but the last two closed in §10:**

- `caves` (the dissolution field is not a declared field), `delta` (the long-term state has no home),
  `transition` (the blend over a region), `earthworks` (the record is bodyless by design) — the four
  refusals above, which were then the four stages between the tally and the `14 of 16` floor;
- Lieth 1975's capacity and Reynolds 1987's flocking (T020), the `--roster` flag printing each
  stage's source (T024), Stratler stream order and lake fill (`SHAPERS.md`'s `hydrology` row);
- the US3 print of a **source** per stage line, and the `checklists/requirements.md` review (T026).
  *(§10 closed the laws and the review; the source-on-the-line and `--roster` stay owed, named there.)*

```bash
cd clean-room
cargo test --offline                        # 102 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # clean
cargo run --offline                         # prints U1/U2/U3 evidence; exit 0
```

**Boundary check (P7):** no file under `crates/` was read, imported or modified while writing U3;
`clean-room/` depends on `sha2` and nothing else (`cargo tree --offline`), and `grep -rn "crates/"
clean-room/src/` finds only the word in documents, never a `use` or a `path`.

---

---

## 10. U3 finished — the record bodies and the in-stage laws (2026-10-11, third pass)

**What landed, and why it was owed.** The first pass left two stage *bodies* owed (`delta`,
`transition`) and four laws owed *inside* stages that already ran (`ecology` produced a province but
not Lieth's capacity or Reynolds' flock; `hydrology` routed water but had no Strahler order and no
lake fill; `hillslope_flux` had a unit test and no consumer). All six are now laws with declared dials
and their own properties. `caves` and `earthworks` still refuse **by decision**, and the difference is
the point: a refusal with a reason is a contract, an owed body is a debt.

**The measured numbers** (from `clean-room/`, the three commands at the end of this section):

| Claim | Measurement (printed) |
|---|---|
| SC-001 one digest, 1/2/4/8 workers | `460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6` at **all four** worker counts |
| SC-002 a moved tick moves it | tick 1 → `7da363a83442494e85baec7864fa3eaddd273e6bc7d115ea943ff3d0d2d8cae5` (differs) |
| SC-003 the tally | **13 of 15 ran, 2 refused by name**: `caves` (`body_owed` — the dissolution field), `earthworks` (`not_a_generator` — the record). The legacy's `14 of 16` was the same rule against the legacy's count; against this roster of fifteen it is `13 of 15` |
| Strahler 1952 in a world | stream order **1–4** over the reference patch: a network that branches, not a ramp |
| Barnes et al. 2014 lake fill | **275 columns hold water**, deepest 0.303 m — a real basin, not a fixture |
| Gilbert 1885 / Shields 1936 delta | deposition **0.1362 m over 96 columns**, capped by the accommodation the standing water leaves |
| Shepard 1968 blend | `blend` spans **0.000–0.995**: the patch crosses a domain boundary, and the tie rule refuses rather than picking |
| Lieth 1975 capacity | up to **165.1 g DM m⁻² yr⁻¹**, the minimum of the two single-factor curves limited by soil |
| Reynolds 1987 flocking | `flock` up to **0.546**, read from the step's noise over the vegetated columns |
| SC-005 conservation | the hillslope flux sums to **−8.674e−18** over a region of magnitude 3.217 — the exchanges cancel exactly over the closed region |
| Declarations | 15 stages, declaration digest `0e2f704345b47814c2fdbad3183a8290a60210bcefe711751c40824a1cc60cb3` |
| Gates | `cargo test` **112 passed**; `cargo clippy --all-targets -- -D warnings` clean; `cargo run` exits 0 (`Clean room OK`) |

**The reference declaration is a world, and a world is dials — recorded rather than tuned silently.**
Making the laws *hold* on the reference patch was itself a declaration change, and the first attempt
failed in an instructive way: with the original dials the tick-0 tally was still 11/15, because
`erosion` refused a patch whose steepest column-to-column step was 2.58 m (a plate-boundary scarp and
an fBm relief sampled below the column scale). The corrections, each a declared value in one table:

1. **The plates carry no signed bias** (`plate_bias_m = 0`) and sit 512 columns apart, so the uplift is
   the *continuous* boundary term. A piecewise-constant plate bias is a discontinuity by construction,
   and a soil law has no finite value on one.
2. **The relief is 5 m over the patch** (`relief_amplitude_m`), with the finest octave above the column
   scale, so no column is a cliff — the world's mountains are the user's to declare, not this bar's.
3. **The patch is 48 columns wide** and its ladder rung is 0 (`ladder_level = 0`, a 32-column cell), so
   the patch crosses a lithology boundary and `transition` has a boundary to blend instead of one
   domain inside one cell.
4. **The critical slope stays the physical `1.0`** (`critical_slope`): the world moved, not the law.

`check_u3_laws` in `src/main.rs` now measures all of this on the generated world — a branching network,
water standing in a basin, a delta that fills it, a blend that crosses, a province that carries
production and agents, and a closed region whose hillslope exchanges cancel — and fails the binary if
any of it stops being true. The tally, the digests and those readings are all printed by `cargo run`.

**Still owed, and named:**

- **`delta`'s avulsion** (Slingerland & Smith 2004) — a decision about the channel over time, which
  needs the state `MISSING-SYSTEMS` §4.1 leaves homeless. The deposition law it would avulse against is
  landed.
- **`caves`' dissolution field** — the occupancy answer's field is still not one of the declared
  fields, so `caves` still refuses by name and says which field it is missing.
- **T023/T024** — the printed roster does not yet carry each stage's **source** on its line, and
  `--roster` is not a flag (the print is unconditional).
- **`docs/MISSING-SYSTEMS.md` is a legacy witness and was left as it stands**; its §4/§5 notes describe
  the legacy tree's own state, not this one.

```bash
cd clean-room
cargo test --offline                        # 112 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # clean
cargo run --offline                         # prints U1/U2/U3 evidence and the law readings; exit 0
```

**Boundary check (P7):** unchanged and re-run — no file under `crates/` was read, imported or modified
while closing these laws; `clean-room/` still depends on `sha2` and nothing else, and `grep -rn
"crates/" clean-room/src/` finds the word only inside documents, never a `use` or a `path`.

---

## 11. U4 slice — the mesh the fields imply, and the work it cost (2026-10-11, fourth pass)

**Created through spec-kit's own workflow, not by hand.** From `clean-room/`:

```bash
.specify/scripts/bash/create-new-feature.sh --number 4 --short-name "u4-mesh-stream-work" \
  "U4 meshing, streaming and work - the geometry a world's fields imply, the resident geometry kept coherent with the world it came from, and the work that produces it budgeted and distributed"
#  → {"BRANCH_NAME":"004-u4-mesh-stream-work"}; specs/004-u4-mesh-stream-work/spec.md copied from
#    .specify/templates/spec-template.md; .specify/feature.json written
.specify/scripts/bash/setup-plan.sh     # → specs/004-u4-mesh-stream-work/plan.md from plan-template.md
.specify/scripts/bash/setup-tasks.sh    # → {"FEATURE_DIR":…, "AVAILABLE_DOCS":[…], "TASKS_TEMPLATE":…}
```

`spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `contracts/u4-seams.md`,
`checklists/requirements.md` and `quickstart.md` were then written **from** `docs/DESIGN-SPEC.md` §4
SPEC-3/SPEC-5 and the **U4** entry of §2 above — never from `crates/voxel-mesh`, `crates/voxel-stream`
or `crates/voxel-work`, none of whose `.rs` files was read (P7: a witness, never a source).

**What landed.** `clean-room/src/mesh.rs` is one module and one capability: the **field-derived**
surface mesh (greedy rectangles over a mask of equal `(layer, height, material)` cells), the level
ladder `0..=2`, the **four typed horizons**, the resident set keyed by the world truth it came from,
the **work ledger** counted where the work happens, and the budget. `src/refusal.rs` gained the four
names (`level_off_ladder`, `horizon_mixed`, `geometry_budget_exceeded`, `geometry_out_of_residency`);
`src/field.rs` gained **one widened seam** (`fields_at`, one evaluation for the three fields, with the
narrow `field_at` implemented over the same checks); `src/main.rs` gained `check_u4`; `src/systems.rs`
now carries U4's real status; and `tests/u4_geometry.rs` drives the whole lifecycle through the public
API against the **reference world**.

**The measured numbers** (from `clean-room/`, the commands at the end of this section):

| Claim | Measurement (printed) |
|---|---|
| SC-001 one digest, 1/2/4/8 workers | `0e7358e45550878105b7ee0ee7f53b1039c1629dbdb059b2e7d333f31f814ed8` at **all four** worker counts |
| SC-003 the merge factor, measured | seed `1592639215`, period `1 s`, tick `0`, chunk `(0, 0, 0)`, level `0`: **1024 columns → 144 quads** in **1031 cells**, merge factor **7.16**, vertices `576`, indices `864` |
| SC-004 the ledger is real | ledger == an independent recount of the returned mesh (`quads`, `vertices = 4 × quads`, `indices = 6 × quads`, `cells`), and `columns_produced` == the field plane's own patch area |
| SC-002 a hit costs nothing | re-ask in the unchanged world: **0 more columns, 0 more quads**, `hits + 1`; the tick-0 entry is **not served** at tick 1 (`false`); the rebuild's digest **moved**; `invalidate` removed **1 of 2** and kept the current one |
| A coarse level reports both numbers | level `2`: read **64 of 1024** produced columns, **27 quads**, digest `28c02ca2…32b2` — a distinct identity |
| SC-005 refusals carry their numbers | `geometry_budget_exceeded` — *this request would produce 1024 columns and the declared budget allows 64* (nothing cached); `geometry_out_of_residency` — *chunk (2, 0, 0) is 64.0 m from the anchor, outside the declared residency horizon 48.0 m*; `horizon_mixed` — *the render horizon 300.0 m is wider than the residency horizon 256.0 m* |
| The four horizons are four types | render `128 m <` simulation `192 m <` residency `256 m <` field `1024 m`, ordered, and mixing two of them does not compile |
| The declared palette, on the record | the reference world's six lithology classes over the starter registry's five admitted ground materials: `0->1 1->1 2->2 3->3 4->4 5->6`, water `5` |
| Gates | `cargo test` **132 lib + 2 end-to-end passed, 0 failed**; `cargo clippy --all-targets -- -D warnings` clean; `cargo run` exits 0 (`Clean room OK`) |

**Three decisions the implementation forced, recorded rather than absorbed** (all three in
`specs/004-u4-mesh-stream-work/research.md`):

1. **The resident set is consulted before the residency horizon** (D5). The first draft gated the
   horizon first, and the end-to-end check caught what that means: moving U2's anchor turned a cache
   *hit* into `geometry_out_of_residency`, which contradicts US2 AC3 — *a view is not truth*. The
   horizon is now a limit on **work** (do not build what will not be kept), and the anchor is not an
   input to an answer. The failing test found a real design fault; the fault is on the record.
2. **One evaluation, three fields** (D2). A mesh must be a statement about **one** run of the world,
   so `fields_at` widens U3's seam instead of the mesher asking three times — and the seam was added
   in the field plane, where fields are produced, not in the consumer that needed it.
3. **A palette thinner than the world's class count is a declared mapping** (D7), printed by the
   binary (`0->1 1->1 …`): the reference world declares six classes and the starter registry admits
   five ground materials, and the honest third option between *refuse a real world* and *invent a
   material* is a monotone mapping on the record. Also recorded: the budget and the worker count
   travel together as `Effort` (D8) — clippy's argument-count limit pushed two knobs into one type,
   which is a better seam than the `#[allow]` that was never an option.

**What U4 still owes, named and printed by the binary:** the **displaced 256× surface** (a non-planar
rectangle must be subdivided and triangulated with normals from the displaced geometry — C-43), the
**multi-chunk stitched mesh** and its seam rule, a **strided field request** (a change to what a patch
*is*, so it belongs in U3's vocabulary), and the **streamed residency policy** (which chunks to keep,
and when to evict — U5/U6's; U4 owns exact invalidation). The sparse voxel octree and `refine` from the
slice table's original wording are the same owed capability seen from the storage side.

**Half of slice 4's own check is still unmet, and named:** the check is *"merge factor and per-chunk
memory re-measured"*. The **merge factor** is measured — `1024 columns → 144 quads`, `7.16` — but the
**per-chunk memory** figure is **not** measured, so `quickstart.md` marks it `unchecked` and the binary
prints it as owed. Estimating a footprint from a struct's fields would be the defect C-46 names rather
than a measurement, so it is owed outright.

```bash
cd clean-room
cargo test --offline                        # 132 lib + 2 end-to-end passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # clean
cargo run --offline                         # prints U1/U2/U3/U4 evidence; exit 0
cargo tree --offline                        # sha2 and its own tree; nothing else
```

**Boundary check (P7):** no file under `crates/` was read, imported or modified while writing U4 —
`cargo tree --offline` still shows `sha2` as the only dependency, `find crates -newermt` shows nothing
touched during this pass, and the only occurrences of the string `crates/` in `clean-room/src` are the
witness lines U1–U3 already carry, never a `use` or a `path`.

---

## 12. U6 slice — the gate, and answers that carry their own evidence (2026-10-11, fifth pass)

**Created through spec-kit's own workflow, not by hand.** From `clean-room/`:

```bash
.specify/scripts/bash/create-new-feature.sh --number 5 --short-name "u6-record-api" \
  "U6 the record and the API layer - what the record holds, how a request enters and what a caller receives back, how an answer carries the evidence for itself, and how a request that cannot be honoured is refused by name rather than answered vaguely"
#  → {"BRANCH_NAME":"005-u6-record-api"}; specs/005-u6-record-api/spec.md copied from
#    .specify/templates/spec-template.md; .specify/feature.json → specs/005-u6-record-api
.specify/scripts/bash/setup-plan.sh     # → specs/005-u6-record-api/plan.md from plan-template.md
.specify/scripts/bash/setup-tasks.sh    # → FEATURE_DIR …/005-u6-record-api
#    before the documents were written: research.md ✗  data-model.md ✗  contracts/ ✗  quickstart.md ✗
#    after:  research.md ✓  data-model.md ✓  contracts/ ✓  quickstart.md ✓
#    (check-prerequisites.sh --json reports the same four)
```

`spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `contracts/u6-seams.md`,
`checklists/requirements.md` and `quickstart.md` were written **from** `docs/DESIGN-SPEC.md` §2 F1/F2 and
the **U6** entry of §2 above — never from `crates/voxel-package` or `crates/voxel-mcp`, neither of whose
`.rs` files was read (P7: a witness, never a source).

**What landed.** `clean-room/src/bridge.rs` is one module and one capability — **a request and its
checkable answer** (§14 later split it along the seams it already had; the public surface is unchanged): a **closed, bijective tool table** of five verbs (`truth`, `fields`, `geometry`,
`held`, `record`) over four owners, each row declaring its arguments, the owner seam it dispatches to,
whether it may change the record, and the checks it runs; a **canonical request line** that is the
request's identity, so two spellings are one request; an **answer** whose digest is a pure function of
what it *describes* (identity, digest, counts — never the work that produced it); the **claims** an
answer makes, each re-checked against the owner it describes; the **checks** that ran, named with their
outcome and subject; the **effect** as a delta of the owners' own counters; and the **receipt** that
carries all of it. `src/chain.rs` gained one event kind at the record's owner (`geometry_derived`, in the
`record` namespace — an API layer that invented its own event vocabulary would be a second record);
`src/refusal.rs` gained the five names (`tool_undeclared`, `argument_undeclared`, `argument_absent`,
`argument_malformed`, `claim_unverified`); `src/main.rs` gained `check_u6`; `src/systems.rs` now carries
U6's real status; and `tests/u6_bridge.rs` drives the whole lifecycle through the public API against the
**reference world**.

**The refusal split is the load-bearing decision.** A request that does not parse against the declared
table is refused **before** the world or the record is touched — nothing is written, because a malformed
line is not an event about anything. A request that parses but cannot be honoured by the world is refused
by the **owner's own name** and that refusal is **recorded**, because it is a fact about the world: the
chain grows by exactly one `refusal_recorded` event naming the refusal and the request that asked it.
Recording can itself fail — a payload that cannot ride the tab-separated wire form — and then the returned
refusal names **both** facts rather than dropping either. (§20 moved a refusal's `detail` out of that
payload for the same reason the chain refuses a tab: the payload is hashed into the record's identity.)

**The measured numbers** (from `clean-room/`, the commands at the end of this section):

```text
U6 tool table: 5 declared verbs, closed and bijective
U6 seed=1592639215 tick=0 repeat [`geometry chunk=1,0,0 level=0`]: one answer digest
  25a485a539173405568d4a003b12f3121f46b5df3e9bd8c4ac6c7392059324be (identical: true); the first did
  1024 columns of work and appended true, the second did 0 and appended false (effects differ: true);
  the record moved once for the change: true
U6 parse refusal by name: [argument_malformed] `chunk` is declared as a chunk: `0,0` is not one
  (the record is untouched: true)
U6 world refusal by name: [geometry_out_of_residency] chunk (9, 9, 0) is 407.3 m from the anchor at
  (0, 0, 0), outside the declared residency horizon 256.0 m -> exactly one event recorded = true,
  naming it = true, nothing cached = true
U6 correspondence: the archived receipt verifies in the world it was made in = true; against tick 1 it
  refuses [claim_unverified] = true
U6   claim_unverified: claim `truth_is`: recorded seed=1592639215 period=1s tick=0, found
  seed=1592639215 period=1s tick=1
U6   claim_unverified: claim `mesh_is_held`: no geometry is held for chunk (1, 0, 0) at level 0 in
  this world
U6 read, not recomputed: evicting 2 stale entr(ies) leaves the archived answer's held-geometry claim
  failing while its truth claim still holds (true)
U6 record: 4 events, head 238b087ff22258913391d371c0a0e9b464838771d34f0f56cf3cca2b439a15c8,
  verifies = true, a `record` read verifies = true and changed 0 event(s)
```

The `geometry` receipt's mesh digest is U4's own printed digest for the same world, chunk and level
(`0e7358e4…814ed8`) — the gate read U4's seam and derived nothing of its own. Both reads that touch the
record print `record 1 -> 1 events, appended nothing`: a read is not a change.

**Two design decisions the pass had to take, recorded rather than inherited.**

1. **The claims are re-checked *inside* `call`** (`specs/005-u6-record-api/research.md` D3). A receipt
   is handed out only while its evidence holds *now*, so `claims_hold` is a load-bearing check rather
   than a re-verification a caller might never run; a claim that does not hold is recorded like any
   other fact about the world. `Receipt::verify` remains the *caller's* check, against whatever owners
   the caller supplies — which is what makes the same receipt provable against the world it was made in
   and refusable against a world that moved.
2. **A field's claim is re-evaluated; a mesh's claim is read** (D3 again). A field has no stored form —
   the world is a function of `(seed, period, tick)` and a function is checked by evaluating it — while
   recomputing a mesh would be a second geometry path (SPEC-3). The difference is measurable: eviction
   fails `mesh_is_held` alone, and the truth claim still holds because the world did not move. That is
   the spec's own edge case, and it is the mission's "correspond to the world rather than be recomputed
   from nothing" made checkable.

**Ownership is honest, and checked.** The chain is the record (the gate appends through `Chain::append`,
the one writer, and reads its head, events and wire form); the world owns truth (the gate reads
`World::truth_digest` and exposes `world_mut` so a caller moves it through `World::with_tick`); U4 owns
geometry (the gate holds a `Resident` and a `WorkLedger` and derives only through `Mesher::derive`, with
`Resident::invalidate` as the caller's eviction). U6 keeps **no** second copy of any of it: the one
capability it needed below it — an event kind — was added at the owner, in `chain.rs`.

```bash
cd clean-room
cargo test --offline                        # 139 lib + 2 U4 + 6 U6 end-to-end passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # clean
cargo run --offline                         # prints U1/U2/U3/U4/U6 evidence (50 U6 lines); exit 0
cargo tree --offline                        # sha2 and its own tree; nothing else
find crates -type f -newer clean-room/src/bridge.rs   # 0 files: nothing under crates/ was touched
```

**Boundary check (P7):** no file under `crates/` was read, imported or modified while writing U6 —
`cargo tree --offline` still shows `sha2` as the only dependency, `find crates -type f -newer
clean-room/src/bridge.rs` returns **0**, and the only occurrences of the string `crates/` in
`clean-room/src` are the witness lines the module headers carry (`bridge.rs` names the two directories it
must **not** copy from), never a `use` or a `path`.

**What U6 still owed, named and printed by the binary *(corrected in §18: the gate's first slice has
since landed; what remains is listed there)*:** the **content gate** — `admit`/`inspect`
returning `Established` (never a bool), the document record and its compression, the calibrated
structural judge, the ballot, the crate-by-axis census with `Unchecked` never a pass, the permission
policy matrix, the recipe format and the API document; the **bijection of the transport surface against
the legacy action vocabulary** (the transport itself landed in §13; the bijection is a witness check P7
forbids reading the legacy for); the **by-reference verbs** (the plan's Lane B rule — `path + digest`,
never bytes — needs an argument kind this slice does not declare); and a **streamed residency policy**
(which chunks to keep and when to evict: U4 owns exact invalidation, which the gate exercises, not the
policy). No stub was written for any of them: a plausible `Established` from a stub would be exactly the
invented default P2 forbids.

---

## 13. U6 slice, second pass — the table served over stdio (2026-10-11, sixth pass)

**What landed.** `clean-room/src/transport.rs` is one module and one capability — **a way in**: one
request line in, one response record out. The line protocol is declared: `status`, `request`, `record`,
`receipt`, `kind`, `detail`, closed by a blank line — and the response's record is the record **after**
the call, so a client sees a world-level refusal recorded (the record grows by exactly one) while a line
that did not parse writes nothing. The transport owns **framing only**: `serve` takes `&mut Gate` and
has nothing else to answer from, and it refuses what cannot cross intact — a request carrying a tab,
newline, carriage return, `~`, `;` or `|` is `payload_not_wirable` **before** the gate sees it (so it is
not an event about the world), echoed as `-` because that line is exactly the one the response cannot
quote. `reference_gate` is the **one** place the reference declaration (palette, ladder, horizon,
identity) is written down, and `check_u6` now builds its gate from it, so the binary's evidence, its
`serve` mode and the tests cannot drift into three similar worlds. (§14 moved that declaration out of
the framing into `src/reference.rs`, where it belongs; the framing keeps `serve`.) The binary gained its `serve` mode
(`cargo run -- serve`: the protocol on stdin/stdout and nothing else on that stream; the session's own
count goes to stderr), and its evidence now drives a **real child process** of itself. The protocol is
declared in `clean-room/specs/005-u6-record-api/contracts/u6-transport.md`, and
`clean-room/tests/u6_transport.rs` drives the real binary as a client the same way.

**The measured numbers** (from `clean-room/`, the commands at the end of this section):

```text
(stderr, the served session's own count)
serve: 7 request(s): 5 answered, 2 refused by name
U6 transport: `serve` — a separate process answered 5/7 request(s) with true correspondence; clean exit = true, every request sent = true
U6   world refusal [geometry_out_of_residency] chunk (9, 9, 0) is 407.3 m from the anchor at (0, 0, 0), outside the declared residency horizon 256.0 m -> recorded (the record grew by one: true)
U6   parse fault   [tool_undeclared] "abracadabra" is not one of the 5 declared verbs: truth, fields, geometry, held, record -> writes nothing (the record held: true)
U6   repeat      one answer digest 4c4da3299efb79daf7252761e5cbacea366b1bc91cdac135f5768892323bff1b (identical: true); the first appended exactly one event, the second did 0 columns and appended none (one append: true)
U6   correspondence: every served receipt verified against this process's own world, resident and record = true; the served record is this process's record = true; world refusal named = true, parse fault named = true
```

Every line above is the binary's own output, unedited. The parse fault is refused **before** the gate
sees it and writes nothing, while the world refusal is a request the gate *did* take and the chain
recorded — one event, named — which is the split the transport must not blur into a single "failed".

**Correspondence, not trust, is the load-bearing decision.** A served receipt is the value the gate
handed out, not a summary of it: the parent replays the same calls in its own gate and verifies each
served receipt's claims against its own owners, compares the served answer digest with the one it
derived, and compares the record length and head with its own after every step. Nothing is believed
because it arrived over a pipe — which is what makes the same answer provable on both sides of the
boundary.

**Ownership is honest, and checked.** The transport holds no truth, no record and no geometry; it
borrows the gate. The one declaration it shares — the reference world — is a declaration, not a copy of
state, and a served session's record opens **empty**, so what a client reads back is exactly what its
own requests recorded.

```bash
cd clean-room
cargo test --offline                        # exit 0: 145 lib + 2 U4 + 6 U6 bridge + 3 U6 transport; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 55 U6 lines; "Clean room OK"
cargo run --offline > a 2> b; cargo run --offline > c 2> d; diff a c
                                            # stdout byte-identical across runs; the only stderr
                                            # difference is Cargo's own build-timing line
cargo run --offline -- serve                # one line in, one response record out on stdout
cargo tree --offline                        # sha2 and its own tree; nothing else
find crates -type f -newer clean-room/src/transport.rs   # 0 files: nothing under crates/ was touched
```

**Boundary check (P7):** no file under `crates/` was read, imported or modified while writing the
transport — `cargo tree --offline` still shows `sha2` as the only dependency, `find crates -type f
-newer clean-room/src/transport.rs` returns **0**, and the only occurrences of the string `crates/` in
`clean-room/src` are the witness lines the module headers carry, never a `use` or a `path`.

**What U6 still owed, named and printed by the binary *(corrected in §18; the transport fixes below
changed nothing about it)*:** the **content gate** — `admit`/`inspect`
returning `Established` (never a bool), the document record and its compression, the calibrated
structural judge, the ballot, the crate-by-axis census with `Unchecked` never a pass, the permission
policy matrix, the recipe format and the API document; the **bijection of the served table against the
legacy action vocabulary** — a witness check P7 forbids reading `crates/voxel-mcp` for; the
**by-reference verbs**; and a **streamed residency policy**. No stub was written for any of them.

---

## 14. Structure — where each concern lives (2026-10-12, the shape pass)

**Shape only, no behaviour.** 187 lines of stdout, 55 of them U6's; 156 tests (145 lib + 2 U4 + 6 U6
bridge + 3 U6 transport) pass; `clippy --all-targets -- -D warnings` is clean; `cargo run` exits 0; and a
hand-driven `serve` session answers exactly as before — `truth` still `146ff7bf…`, the derivation still
records one event, the undeclared verb still writes nothing. The **one** line of stdout that changed is
the registry's own interface row, which now names the modules the code actually has.

Three files had each grown past one concern: `bridge.rs` (2145 lines) held the table, the canonical
line, the receipt, its wire form *and* the gate; `transport.rs` held the framing **and** a whole
declared world; `main.rs` (1252 lines) held two entry points and every system's measurements. The seams
already existed as banners inside those files, which is the tell — the files had simply not been cut
along them.

| file | owns |
|---|---|
| `bridge/mod.rs` | the façade: the module's argument and the `pub use` of the five below — every `crate::bridge::X` path is unchanged |
| `bridge/table.rs` | the declared surface: `ArgKind`/`Argument`, `CheckName`/`CheckOutcome`/`Check`, `Verb`/`Tool`/`TOOLS` |
| `bridge/request.rs` | one declared line, and its canonical identity |
| `bridge/receipt.rs` | `Answer`, `Claim`, `Effect`, `Receipt`, `Owners` — the evidence an answer carries |
| `bridge/wire.rs` | the receipt's **external form**: the version, the separators, the round trip |
| `bridge/gate.rs` | the machinery: the world, the record, the resident geometry, `call`, and `Answered` |
| `reference.rs` | the one reference world two processes compare answers through (`identity`, `ruleset`, `gate`, `serve_stdio`) |
| `save/mod.rs` | what a save **is**: `Ruleset`, `StreamSeed`, `SaveIdentity`, `Save`, and its writer/parser — every `crate::save::X` path is unchanged |
| `save/layout.rs` | the byte level: `MAGIC`/`VERSION`, the one encoder, the bounded `Reader`, and `read_header` |
| `save/load.rs` | the correspondence checks: `Save::load`, `Loaded`, `LoadReport` |
| `save/tests.rs` | the layout, the parser and the loader driven directly |
| `transport.rs` | framing, and only framing |
| `evidence.rs` | one measurement per built system |
| `main.rs` | the two ways in (78 lines) |

Two ownership rules came out of the cut, and later passes should keep them: **the framing reads the
wire's separators** — `REQUEST_FORBIDDEN` composes `bridge::wire::{WIRE_PART, WIRE_ITEM,
WIRE_CHECK_FIELD}` instead of respelling `~ ; |`, with a comma deliberately legal because `chunk=0,0,0`
is a request — and **the framing does not know which world it serves** (`serve(&mut Gate, …)`, with the
declaration in `reference::gate`).

**What it buys.** A client that only reads answers — a transport, a verifier, a test — needs
`bridge::receipt` and `bridge::wire`; only the server side needs `bridge::gate`. A change to the receipt
wire lands in one file that owns nothing else; a change to which world a session serves lands in
`reference.rs`. And the acceptance measurements are no longer reachable only by running the binary:
`evidence` is part of the library, beside the registry it measures.

**Extended 2026-10-12 (the save subsystem's own shape pass).** `src/save.rs` was 1,064 lines: the
layout, the vocabulary, the writer, the parser, the loader and 434 lines of tests in one file. It is now
four, cut along the questions it was already answering: `layout` (*what are these bytes*), the module
root (*what is a save made of*), `load` (*is this save of the world I am loading*), and `tests`. The
three ownership rules this pass adds, for later passes to keep:

- **The byte level does not know the schema.** `layout` reads and writes fields; it never asks whether a
  save is of a particular world, and `load` never reads a byte. Its items are `pub(super)` — visible to
  the save subsystem and to nothing else.
- **The ruleset identity has one owner.** `reference::ruleset()` joins `reference::identity()`, and the
  binary's evidence and both test files read it from there; the literals that used to spell
  `Ruleset::new(7, 3)` at each call site are gone. A **mismatched** ruleset is still spelled where it is
  meant — one literal per test file, as the value the refusal exists for. `specs/006`'s `plan.md` tree
  was updated to the new paths; its `tasks.md`, checklist and quickstart keep the paths as they were
  when their checks ran, which is what a task log is.
- **`Save::load(expected: Ruleset)` keeps its parameter.** A save stays portable across mod changes
  *because* it names the identity it was made under, so the caller must say which identity it is loading
  into; defaulting that would silently accept a save of another world.

**Shape only, no behaviour:** `cargo test --offline` still passes **186** (169 lib + 2 U4 + 6 U6 bridge +
5 U6 transport + 4 U9) and `clippy --all-targets -- -D warnings` is exit 0. The cut left stdout
byte-identical — same **240 lines, same `sha256 5ae54082…2ab71`** — and then **one line changed on
purpose**: the U9 refusal line named the file that pins the two unreachable refusals, and that file is
now `src/save/tests.rs`, so the printed path moved with it (stdout `sha256 de0d0c23…` afterwards, 240
lines, identical across two runs). Every measured number is unchanged; the only edited text is a path.

**Boundary check (P7), re-measured:** `cargo tree --offline` still shows `sha2` as the only dependency,
`find crates -type f -newer clean-room/src/reference.rs` returns **0**, and the only occurrences of the
string `crates/` in `clean-room/src` are the witness lines the module headers carry.

---

## 15. The two defects the audit named, fixed at the cause (2026-10-12)

**What was wrong.** (1) Every session client wrote its whole session before reading any of the answers:
`talk` in `tests/u6_transport.rs` and the binary's own transport evidence in `src/evidence.rs`. Seven
lines never mattered, but the shape is the one later passes inherit, and once the server's answers fill
the pipe buffer — 478 bytes per `truth` answer against a buffer of a few tens of kilobytes — the client
blocks writing and the server blocks writing, and neither ever moves: a **hang**, not a failure. (2)
`parse_response` accepted any text as an `err` response's `request` echo and then **discarded** it, so a
response could claim to be about a line it never quoted and no caller could tell.

**What changed.** Clients now send one request and read its answer before the next
(`transport::read_response_for` is the shared reader and `Response::is_about` is the rule, with the
per-response check in the evidence client and in `Client::talk`). `Response::Err` **keeps** the echo it
is about; `parse_response` requires it, and refuses one carrying the framing's own characters — a line
that could not have crossed — or none at all, both `malformed_wire`. An `ok` answer stays anchored
where it was: the receipt's own canonical request, so a client that sent a non-canonical spelling still
recognises its own answer. No capability was added and no answer changed.

**Measured** in the shell, 4000 requests through the real binary (one answer is 478 bytes):

```text
requests: 4000 lines, 24000 bytes;  one answer: 478 bytes
a client that reads as it writes:    1 912 000 bytes of answers, status 0, elapsed 1 s
a client that defers reading by 8 s: status 0, elapsed 9 s   (the server is blocked the whole 8 s)
```

And the shape itself, measured by a throwaway copy of the **old** client run once and then deleted —
12 000 requests, which is 72 KiB of requests and megabytes of answers, more than a pipe buffer on either
side:

```text
the old shape, write everything then read:  0 answers in 20 s (blocked; the bound killed it)
the new shape, read while writing:          12 000 answers, 1.3 s, exit 0
```

With the old shape the client never reaches its first read, because it is still waiting for its own
writes to finish: the session does not finish late, it does not finish at all. The new test pins that
with a **bounded** wait — 12 000 requests, 60 s, driven on its own thread — so a deadlock fails loudly
with a name instead of parking the suite forever.

```bash
cd clean-room
cargo test --offline                        # exit 0: 149 lib + 2 U4 + 6 U6 bridge + 5 U6 transport; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; byte-for-byte what it printed before the fixes
```

**Kept as behaviour, not changed:** a blank request line is a **counted request that names no verb**
(`tool_undeclared`, writing nothing) — not ignored, and not an end-of-session marker. It is now declared
in `contracts/u6-transport.md` rather than left to be discovered.

**Boundary check (P7), re-measured:** `cargo tree --offline` still shows `sha2` as the only dependency
and `find crates -type f -newer clean-room/src/transport.rs` returns **0** — nothing under `crates/` was
read or written for these fixes either.

**What U6 still owes *(corrected in §18: the gate's first slice has since landed; the remainder is
listed there)*:** the content gate (`admit`/`inspect` → `Established`, the document
record, the structural judge, the ballot, the crate-by-axis census, the permission policy matrix, the
recipe, the API document), the bijection against the legacy action vocabulary, the by-reference verbs,
and a streamed residency policy.

---

## 16. The decision §2 reserved — the mission's five, admitted as U9–U13 (2026-10-12)

**The gap, measured, before anything was decided.** §2 closes the system list at eight and says a ninth
is *"a decision for the record."* The mission's completeness clause names five systems the eight do not
carry. Read-only against this file, before this pass:

| The mission names | What this plan carried |
|---|---|
| **planet shploingus** | `planet` **0** mentions. One fragment: the `tectonics` shaper, named in `docs/SHAPERS.md` §2 against *Procedural Tectonic Planets* (Kreischer & Bommes 2019), inside U3 |
| **world saves and loads, with their own seed systems** | `save` **1** — the unrelated `--no-save` npm row in §8's generator log; `loads` **0**. The *materials* exist (U2's `WorldIdentity`, `Tick`, `Chain`; U3's determinism), but no surface |
| **civilian organisations for mobs and autonomous players** | `civilian` **4** — all inside U2, and owed there (T016); `mob` **0**; `organisation` **0** |
| **a more complete shell that also uses gholtzap/lintapple** | the shell is U1 (chrome painting) plus U8, which is last; `lintapple` **0** in this file *before this pass* and **1** in the whole repository — `clean-room/specs/001-presentation/plan.md:34`, the **P8** constitution row, status `WIP` / `NEEDS WORK` |
| **the full minimal gameplay tree derived from the mods references** | `gameplay` **0**; `MODS-MAP` **0**; the intake path exists only as U6's owed content gate and `recipe` |

The mission is authoritative over its own completeness definition, so the decision is taken here **on
the record** rather than deferred again. **U9–U13 are admitted.** §2's eight entries are not rewritten
by this section; §16.3 records the one correction this pass makes inside §2.

### 16.1 The five, in §2's entry shape

**U9 — Saves and loads, and the world's own seed systems**

* **unifies:** the **save** (what is written down), the **load** (what reads it back), and the **seed
  systems** — the world seed, the declared period, every deterministic sub-stream's own seed, and the
  configuration/ruleset identity a save must name so that a load is exact rather than approximate.
* **legacy witness (evidence of the contract only):** `crates/voxel-client`'s `World` — *a seed, a tick
  and a configuration held together*, with the tick in the struct **for this reason** (`LEDGER.md`
  C-96.1); `voxel-shapers`' `WorldSpec` (seed, tick, config) and its per-stage seed derivation. No `.rs`
  is read while writing the fresh code (P7); what they *pin* is what is re-derived.
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §1.3 (determinism) and §2 **F1**
  (*"replay inputs recorded, not derived"*), **SPEC-3** (one canonical world authority); `LEDGER.md`
  **C-96.1** (*the seed is a property of a world*; *a save is record + seed + tick*); the determinism
  test `docs/MISSING-SYSTEMS.md` §1 states (*a ticked world and a rebuilt world produce one digest,
  whatever the worker count*); `docs/CIVILIAN-PORT.md`'s H-7.20 line with its `GeneratorIdentity`
  (`seed, ruleset, revision`); `docs/GAIA.md` §11/§5c (a save stays portable across mod changes
  **because each record names its ruleset identity**). **Must not copy:** those `.rs` files, and no
  legacy save format is the contract — if one ever is, it is cited by `path + digest` (P7).
* **interface:** `save` and `load` over declared, exact, byte-round-tripping bytes; a refusal vocabulary
  for a save missing a seed, a configuration field or the ruleset identity, for a load under an identity
  the save does not name, and for a tampered save. Exposing these through the **one** tool table is
  U6's door, named owed.
* **slice:** **9** — first of the five, and the only one whose acceptance instrument already exists and
  has been **measured**: the digest invariant across 1/2/4/8 workers (U3's `460214b1…2fc6`; U4's
  `0e7358e4…4ed8`). **Its first slice is built and measured (§17, 2026-10-12);** what it still owes is
  named at the end of that section.

**U10 — The gameplay tree the mods references imply**

* **unifies:** the ordered set of playable systems the mods references imply — world shape and
  streaming, industrial and content slots, presentation/performance, UI/audio/ecology — as **one
  declared tree of rows**, each owed by a document, a licence verdict and an acceptance check.
* **legacy witness:** `docs/MODS-MAP.md` (**106 references**, one row per reference with its `class` = *the
  clean-repo destination*, and `mapped`/`open`/`refused` per row, including five licence **refusals**; the
  map's header reconciles that count against the 91-URL file it was written from and this repo's 97-URL copy)
  and `docs/MODS-MATERIALS.md`. The legacy `packages/`/`intake/` machinery is the old tree's, not this
  system's.
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md`'s ruling 3 (*"a fully playable basic
  world with all terrain shapers"*) and §3's S-01…S-24 slots; `docs/MODS-MAP.md` §1–§5;
  `docs/SHAPERS.md`. **Must not copy:** any mod's code, and **nothing derived may be implemented from a
  refused reference** — the licence verdict is per reference and no milestone may grant a blanket one.
* **interface:** a **tree of declared rows** — feature, source document (`path + digest`), the licence
  verdict it relies on, the system that carries it (U3–U8), and its own acceptance check — plus a
  refusal for a row whose source is refused or whose verdict is absent.
* **slice:** **10.** Its hard prerequisite is **U6's content gate**: deriving from a reference is an
  *admission*, never a transcription, so `admit`/`inspect → Established` and the by-reference verbs must
  land first. This system is *the* thing that gate blocks.

**U11 — planet shploingus (the planet-scale world)**

* **unifies:** the planet as one world — its scale contract, its addressing, what the regional ladder
  means at planet scale, and the residency policy that lets a planet exist **without being resident**.
* **legacy witness:** nothing inside this repository implements a planet. The outside tree named by
  `docs/PLAN-2026-10-08-cloud-cubes.md` (`F:\planet-shploingus\docs\ART-DIRECTION.md`) is *reference,
  never authority* and is not this repository's to copy; the witness is U3/U4's own measured numbers
  plus `SHAPERS.md` §2's `tectonics`.
* **fresh boundary ✱ may re-derive from:** `docs/DESIGN-SPEC.md` §5's scale contract and SPEC-5 (four
  horizons, computed not copied); `docs/SHAPERS.md` §2; the papers its shapers name.
* **interface:** a **declared scale/extent contract**, plus U4's owed `Octree`, `refine` and the
  **streamed residency policy** promoted from "owed" to the thing this system is judged by.
* **slice:** **11.** *Hard, and named:* **U4's octree + `refine` + the streamed residency policy** — a
  planet can never be resident, so the policy is the mechanism, not a convenience. *And one ruling that
  is the user's:* the 20-level ladder tops out at `32 · 2¹⁹ = 16 777 216` blocks = **16 777.216 km**
  (`DESIGN-SPEC` §1.2), which is **less than an Earth-sized circumference (40 075 km)** — so planet
  scale needs a declared ladder/extent ruling. Coordinates are **not** the limit: signed 32-bit metres
  reach ±2 147 484 km.

**U12 — Civilian organisations for mobs and autonomous players**

* **unifies:** the organisations civilians, mobs and autonomous players belong to and act through —
  membership, authority, the verbs each member may use, and the record of what it did.
* **legacy witness:** `crates/voxel-model/src/civilian/` (the ported civilians' process — `BLD-0221`,
  whose `pub mod civilian;` is **absent** from that crate's `lib.rs`, i.e. written and never compiled
  in) and the government vocabulary in the record (`LEDGER.md`: `Citizen::autonomous`, `grant_authority`,
  `VerbPolicy`, `admit_draft_material`, `set_consequence`), with `docs/GAIA.md` for an autonomous
  non-player population.
* **fresh boundary ✱ may re-derive from:** the Axis/Laya rules quoted in `LEDGER.md` (the seven axes
  and their prohibitions); `docs/GAIA.md` §2–§9; `docs/CIVILIAN-PORT.md` read as a document (the
  inventory and verdict per file); `DESIGN-SPEC` §2 **F11** (actors, governed work) and F8 (a ceiling
  refuses). **Must not copy:** `civilian/**`, `axis.rs`, `laya.rs` source.
* **interface:** an `Organisation` with declared membership and authority, and an actor that **asks**
  the one tool table what it may do (U6's self-description) and whose every act is a governed event on
  the **one** `Chain`.
* **slice:** **12.** *Hard, and named:* **U2's owed civilian/government vocabulary (T016)** — the
  organisation's nouns *are* that vocabulary; **U6's permission policy matrix and self-describing
  protocol** — an autonomous actor that cannot discover the verbs it may use cannot act at all —
  *(2026-10-12, §29: the **matrix** has since landed as the policy chokepoint: an act per row, one
  decision function, deny by default and every denial recorded. What remains in front of this row from
  that clause is the **self-description** — the protocol an autonomous actor would discover the verbs
  through — which this build has no caller for yet)*; and
  **U7** — a mob's choices are axis/laya questions and U7 is `absent` *(superseded 2026-10-12, §25:
  U7 is **built**)*.
* *(2026-10-13, §32: the **first slice of this row is built**, and it is the row's own sentence read
  literally: the being's choice **is** an axis/laya question — the stage question of
  `patches/PATCH-039-the-corpus-beyond-this-tree.md`, over the five declared stages, with the floor its
  charter declares, answered by the **von** classifier reading the being's own drafted act in place. The
  being's need is met through the game's own cellular law — the `hydrology` stage's `elevation` /
  `water_level` cells, walked along `d8_receivers` *(§33's repair, 2026-10-13: **one static evaluation of
  the roster at `Tick(0)`, not a stepping automaton** — the substitution this sentence makes is flagged
  here rather than left to be read as one, and a real stepping cellular automaton on the field plane is
  **owed**)* — and its every act, denial and unmet need is a governed
  event on the one `Chain`. What the row still waits on is exactly what it waited on: U2's T016 vocabulary
  and U6's self-description, neither of which this pass invented.)*

**U13 — The shell completed, and the gholtzap/lintapple gate** *(closes U8; it does not replace it)*

* **unifies:** the mission's "more complete shell" as an acceptance surface — the shell drawn with U1's
  full `shade`/sky, its constants **admitted** rather than literal, and its own conformance gate
  (**P8, gholtzap/lintapple**) satisfied rather than `WIP`.
* **legacy witness:** `crates/shploingus/*`, `crates/voxel-ui/*`, `crates/voxel-client/src/shell/*`, and
  the ported machinery in `docs/SHELL-PORT.md`.
* **fresh boundary ✱ may re-derive from:** `docs/SHELL-PORT.md` (the four-bucket rule for every drawn
  value), `docs/UI-SPEC.md`, `docs/DESIGN-SPEC.md` §3, `portal_simulator.py`'s *structure*, and the
  linter's own published rules. **Must not copy:** `crates/shploingus/`, `crates/voxel-ui/`, `shell/`.
* **interface:** U8's `Mount`-shaped pane tree **plus the gate** — the shell's drawn values and its tool
  surface checked by P8, with the result recorded. A shell that cannot pass its own gate is not
  complete.
* **slice:** **13.** *Hard, and named:* **U1's owed `shade`/sky/visible client** (row 1, unstarted),
  **U6's admitted constants table**, **U8 itself** (`absent`), and **the P8 gate**, which no slice
  currently owns. Last because U8 is *"the acceptance surface for every system above it"*.

### 16.2 The order, system by system

| # | System | Hard prerequisite it waits on | Adjacent, not blocking | Why it sits here |
|---|---|---|---|---|
| **9** | **U9** saves and loads + the seed systems | **none owed** — U2's identity/`Chain`/`Digest` and U3's determinism are built and measured | U6's by-reference verbs and a `save`/`load` row in the closed table (the *document* form of a save). U4's residency is **not** involved: a save is the record, not the resident set (F1) | the only one of the five that can start today, and it turns P3/C-96's *"a ticked world and a rebuilt world produce one digest"* into a test of a written artifact that every other system can then be pinned to — and it did: **first slice built 2026-10-12 (§17)**, with the document form and the table rows still owed — **and both of them built the same day (§28)**, so what is owed now is a save of a resident set, compression and a versioned migration path, exactly as this row's *adjacent, not blocking* column predicted |
| **10** | **U10** the mods-derived gameplay tree | **U6's content gate** (`admit`/`inspect → Established`) + the by-reference verbs + the recipe/content format — all owed | U4's residency (content that streams), U7 (content that chooses) | it is the repository's stated first target (*"a fully playable basic world"*, DESIGN-SPEC ruling 3), its disposition map already exists (`MODS-MAP`), and it becomes startable exactly when the U6 slice already in flight finishes |
| **11** | **U11** planet shploingus | **U4's octree + `refine` + the streamed residency policy**; and a **scale/extent ruling** (the ladder's 16 777.216 km is short of a planet) | U9 (pinning a planet-scale state to test it), U5 (instruments to read it) | residency at planet scale is mechanical once U4 is complete, but the extent ruling is the user's and the system cannot be judged before it |
| **12** | **U12** civilian organisations | **U2's T016 vocabulary** + **U6's self-description** (the **policy matrix** landed 2026-10-12, **§29**; its own remainder is the self-describing protocol) + **U7** (**built**, §25) | U9 (an organisation's history is a record), U11 (a population needs a world) | it waits on two systems at once rather than three *(corrected 2026-10-12, §29: U7 is built and the matrix landed, so what it waits on is U2's vocabulary and U6's self-description)*, so it cannot precede them without inventing the vocabulary they own *(2026-10-13, §32: the first slice is built, still on its two owed prerequisites — the prerequisite list has not moved, and the slice names what it read in their place rather than inventing either)* |
| **13** | **U13** the shell completed + its lintapple gate | **U1's remainder** (row 1) + **U6's admitted constants table** + **U8** (`absent`) + **the P8 gate**, which no slice owns | U9–U12, every one of which the shell is the acceptance surface for | last by §2's own reasoning: *"the only place a person sees the world… the acceptance surface for every system above it"* — completed first, it would be judged against nothing |

**Why U9 is first.** It is the only one of the five whose prerequisites are already met *and* whose
acceptance instrument already exists and has been measured. The plan's own determinism claim is that a
ticked world and a rebuilt world produce **one digest whatever the worker count**; U3 measured exactly
that (`460214b1…2fc6` at 1/2/4/8) and U4 repeated the shape (`0e7358e4…4ed8`). A save/load round-trip
turns that from an assertion about the generator into a test of a **written artifact** — and it is what
lets the other four be judged at all: the planet's residency, an organisation's history and the
content tree's rows all have to be pinned to a world to be measured.

**The content gate is adjacent to U9, not a prerequisite of it — stated plainly, because the question
was asked directly.** A save in its **replay form** — record + seed + tick (H-7.20) — needs U2's
`Chain`, `WorldIdentity`, `Tick` and `Digest` (built) and U3's determinism (built and measured); it does
not need `admit`/`inspect`, the document record, the structural judge, the ballot, the census, the
policy matrix or the API document. The gate becomes a **hard** prerequisite only for the *other* form of
a save: one that crosses the API as a **document consumed in place by reference** (`path + digest`).
That is why spec 006 names the replay form as its slice and the document form as a named, owed
extension — and why **U10, not U9, is the system the content gate truly blocks.**

### 16.3 Corrections this pass records (not quiet edits)

| # | Where | Was | Now | Why |
|---|---|---|---|---|
| 1 | §2, U6's `slice:` field | `6` | `5` — row 5 of §5, taken with U5 | §5's rows are the ordered pass list and it puts U6 with U5 in row 5. From U6 onward §2's fields ran one ahead of §5's rows, which made "the next slice" ambiguous. U1–U5's fields are unchanged, and so is every word of the eight entries' substance |
| 2 | §2, U7's `slice:` field | `7` | `6` — row 6 of §5 | the same off-by-one |
| 3 | §2, U8's `slice:` field | `8` | `7` — row 7 of §5 | the same off-by-one |
| 4 | §5's closing line | *"The next slice is 0 → 1"* | the line is kept, with a dated correction beneath it | it was written in pass 1; passes 2–6 ran ahead of it (U2, U3, U4, then U6's gate and transport). Row 1's remainder (U1 complete) and row 5's first half (U5) are still owed |
| 5 | §6's status sentence | *"U5–U8 are `absent`"* | marked superseded in place, with the correction recorded inside §6 | U6 landed (§12, §13, then §15's two fixes). Of U5–U8, U5, U7 and U8 remain `absent` |

**Row 8 of §5 is a standing gate, not a blocker.** The Lane A/B/C measurement re-run is not in front of
rows 9–13; it is repeated after each of them, which is what "lines-named-by-a-record reach the bar"
means as a criterion.

### 16.4 What this pass created

One spec, through **spec-kit's own workflow** as specs 003–005 were made — the same script
(`clean-room/.specify/scripts/bash/create-new-feature.sh`, spec-kit v1.1.3), the same template
(`.specify/templates/spec-template.md`), and `specs/006-world-saves-and-seeds/spec.md` written from it
with `.specify/feature.json` pointed at the new feature — for **U9**, the system this order puts first.
Its `checklists/requirements.md` is the quality checklist `/speckit-specify` requires. **No
implementation was written and none was asked for:** `plan.md` and `tasks.md` are the *plan* step's
artifacts and are deliberately absent, so no implementer can start from a spec that has not been
planned. Nothing under `crates/` was read or written, and no clean-room source was touched.

**Follow-up, same date — the plan and tasks steps then ran, and the first slice with them (§17).**
`plan.md`, `tasks.md`, `research.md`, `data-model.md`, `contracts/u9-seams.md` and `quickstart.md` now
exist for 006, made through spec-kit's own `/speckit-plan` and `/speckit-tasks` as specs 003–005 were
made, and the row this section left unchecked in the requirements checklist was settled by running its
instruments rather than by reading them. The section above is left as written: it records what *that*
pass did, and §17 records what the next one did.

---

## 17. U9 slice — a written world that reads back as itself, and the seed systems in the plural (2026-10-12, seventh pass)

**Created through spec-kit's own workflow, not by hand.** From `clean-room/`:

```bash
.specify/scripts/bash/setup-plan.sh    # → specs/006-world-saves-and-seeds/plan.md from plan-template.md
.specify/scripts/bash/setup-tasks.sh   # → {"FEATURE_DIR":…, "AVAILABLE_DOCS":[…], "TASKS_TEMPLATE":…}
```

`plan.md`, `tasks.md`, `research.md`, `data-model.md`, `contracts/u9-seams.md` and `quickstart.md` were
then written **from** `docs/DESIGN-SPEC.md` §1.3 and §2 F1/SPEC-3, `LEDGER.md` C-96.1 and H-7.20, the
**U9** entry of §16 above, and the determinism bar `docs/MISSING-SYSTEMS.md` §1 states — never from
`crates/voxel-client`'s `World` or `crates/voxel-shapers`' `WorldSpec`, neither of whose `.rs` files was
read (P7: a witness, never a source). The row §16.4 left unchecked in
`specs/006-world-saves-and-seeds/checklists/requirements.md` — *"feature meets measurable outcomes"* —
is settled here by running its instruments, and the file now says which instrument decided which
criterion.

**What landed.** `clean-room/src/save.rs` is one module and one capability: the **exact length-framed
encoding** (a header, the declared stream table, the chain in its own wire form, a trailer naming every
byte) with a parser that checks integrity **before** it interprets anything; `Save::load` → `Loaded`,
which is the only way a world comes back and exists only when every check passed; `SaveIdentity`,
`StreamSeed`, `Ruleset` and `LoadReport`. `src/world.rs` gained **`World::stream_seed(domain)`** — the
per-domain seed, at U2's owner; `src/refusal.rs` gained six names (`not_a_save`, `save_truncated`,
`save_tampered`, `seed_undeclared`, `identity_mismatch`, `record_diverged`); `src/reference.rs` gained
`identity()`, the **one** declaration of the reference world's identity, so the binary's evidence, its
session and its tests cannot drift into three similar worlds (C-46); `src/main.rs` gained `check_u9`;
`src/systems.rs` now carries U9's real status; and `tests/u9_saves.rs` drives the whole thing through
the **public API** and a **real file**.

**The measured numbers** (from `clean-room/`, the commands at the end of this section):

| Claim | Measurement (printed) |
|---|---|
| SC-001 a written world reads back as the same world | write → parse → `Save::load` → `32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539`, and the live world at the same tick digests **equal**; the previous tick **differs** |
| SC-001 the bytes are exact | **465 bytes** at a real path, the file and the re-encoding of the loaded world are the **same bytes** (`= true`), and two saves of one world are one byte string |
| SC-002 the seed systems, plural | **five distinct** per-domain seeds — `terrain 110187191988996982`, `geology 85828532078282826`, `climate 6158460919897384818`, `hydrology 17877078227061120690`, `detail 164424443644523746` — each `= the world's own: true`, `5 of 5 declared` |
| SC-003 the record is carried, not mirrored | `2` event(s), head `6d0ae091…acb23`, `declared and carried agree = true`; a coherent save declaring another record is refused as `record_diverged`; an **empty** record is still a record |
| SC-004 a save that cannot be trusted is refused by name | `not_a_save`, `save_truncated` (a cut at 100 bytes names the extent), `save_tampered` (a flipped byte, both digests), `identity_mismatch` (ruleset 7 vs 8, both named) — all `= true`, and a pass with **4 faults collected at once** |
| SC-005 determinism, whatever the worker count | the rebuilt world's generation digest is `c331cf554a47e777254d32b673079d7abc0eca624da2e4afd705160a02a5c74b` at **1, 2, 4 and 8** workers — one value — and `rebuilt == ticked = true` |
| SC-006 no re-derivation, and no partial load | `truth_digest` is **derived** on load and never read from the file; a refused load produces **no world and no record** (another ruleset, and a declared period of zero refused by U2's own rule) |
| The tick is not the seed | two saves at consecutive ticks share the seed and the period and differ only in the tick (asserted twice: in the crate and through the public surface) |
| The refusal vocabulary is still closed | `RefusalKind::COUNT` is **50** and `ALL.len() == COUNT`; the six new names are `ALL`'s, not a side list |
| The registry is honest about what exists | **13** systems, U9 through U13 carried by name — five of them `absent` or `in progress`, never silently dropped |
| Gates | `cargo test --offline` **166 lib + 2 U4 + 6 U6 bridge + 5 U6 transport + 4 U9 = 183 passed, 0 failed**; `cargo clippy --offline --all-targets -- -D warnings` exit 0; `cargo run --offline` exit 0, **byte-identical stdout across two runs** (240 lines, `sha256 5ae54082…2ab71`) |

**Two decisions the implementation forced, recorded rather than absorbed** (both in
`specs/006-world-saves-and-seeds/research.md`):

1. **The per-domain seed is a new declared number, not a re-rooting of `stage_seed`** (D2). The obvious
   implementation — fold the domain into the existing per-stage derivation — would have changed every
   stage's noise field and so **moved the already-published U3 digest (`460214b1…2fc6`) and U4's
   (`0e7358e4…4ed8`)**. A save format is allowed to add a number; it is not allowed to silently
   invalidate two measured results. The stream seed is therefore its own pure function of
   `(seed, period, tick, domain)` at U2's owner, and the five values above are new and stable.
2. **The writer will not emit a save it would refuse to load.** That asymmetry is deliberate, and it is
   said out loud rather than hidden: `seed_undeclared` and `record_diverged` are **not reachable through
   the binary's own writer** (the binary prints four of the six names and names the other two and where
   they are pinned), because the alternative — a writer that can produce a save its own loader rejects —
   would make the round-trip bar unfalsifiable. Both are pinned by crate tests that construct the bytes.

**What U9 still owes, named and printed by the binary:** the **document form** of a save (a save
consumed in place by reference — `path + digest` — which is the one place U6's content gate becomes a
**hard** prerequisite, exactly as §16.2 ruled); **`save`/`load` as rows of U6's one tool table** (the
table is closed at five verbs today, and widening it is U6's door, not U9's); **a save of a resident
set** (a save is the record, not the residency — F1); **compression**; and a **versioned migration
path** (H-7.20's `seed, ruleset, revision` is carried, so a migration has something to read, but no
migration exists).

**The four graded dimensions, stated plainly:**

| Dimension | Verdict |
|---|---|
| **Correctness** — a written world reads back as the same world | **graded and passed**: one `truth_digest` for the loaded and the live world, the record byte-identical, the bytes exact in both directions |
| **The named claim, in the plural** — the world's own *seed systems* | **graded and passed**: five domains, five distinct values, each checked as the world's own, and both directions of the declaration checked (a missing domain **and** an undeclared one) |
| **Determinism** — the plan's P3 bar | **graded and passed**: one generation digest at 1/2/4/8 workers, and byte-identical save files; it is now a statement about a written artifact rather than only about the generator |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: every ticked row names its check; the owed list is named and printed rather than half-built; the two unreachable refusals are declared; and **nothing under `crates/` was read or written** (P7) |

```bash
cd clean-room
cargo test --offline                                 # 166 lib + 2 + 6 + 5 + 4 = 183 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings  # exit 0
cargo run --offline                                   # prints U1–U4/U6/U9 evidence; exit 0
cargo run --offline > a.txt; cargo run --offline > b.txt; cmp a.txt b.txt   # identical
cargo tree --offline                                  # sha2 and its own tree; nothing else
```

**Boundary check (P7):** `find crates -newermt "-6 hours"` returns **nothing** — no file under `crates/`
was read, imported or modified while writing U9. `clean-room/Cargo.toml` declares `[workspace]` with
itself as the root, so the legacy workspace at the repository root (
`members = ["crates/…"]`) does not claim it and cannot be built by, or coupled to, this project; it
declares **no** `path` dependency on any crate under `../crates/`. The only occurrences of the string
`crates/` in `clean-room/src` are the witness lines U1–U4 already carry, never a `use` or a `path`.

---

## 18. U6's content gate, first slice — a declared row, admitted in place by reference (2026-10-12, eighth pass)

**Why this, and why now.** §16 row 10 names **U10** (the mods-derived gameplay tree) and its hard
prerequisite in one sentence: *"deriving from a reference is an **admission**, never a transcription,
so `admit`/`inspect → Established` and the by-reference verbs must land first. This system is the thing
that gate blocks."* §2's **U6** entry names the gate's interface; §5 row 5 names its bar — *an unadmitted
package refused by name; `Established::ChecksumOnly.establishes_identity() == false`*; and the record's
own decision on intake is `LEDGER.md` **§55.3**: integrity and identity are **two different
statements**, a `ChecksumOnly` payload is admitted **and reports that no identity was proven**, pinning
is the most this repository can honestly offer, `admit` collects **every** problem rather than the
first, and a refused package is **not partially admitted**. This slice is built from those three
documents; no `.rs` under `crates/voxel-package/` was read (P7), and nothing the documents leave
undesigned was filled in — it is named owed, below.

**What landed.** `clean-room/src/intake.rs` is one module and one capability: **a declared row is
admitted, or refused by name, on the record of what was and was not established.** A row *points at*
its document — a path and the digest it must hash to — so the gate reads it **where it lives** and no
declaration ever carries a body. The row declares the licence verdict it relies on, the carrier that
carries it, its own acceptance check, and its claim (`ChecksumOnly`, or `Pinned { signer, digest }`).
The gate decides five **demanded** axes (`feature`, `source`, `licence`, `carrier`, `acceptance`),
**reports** a sixth (`identity`), and returns `Established` — which is **not a bool**. `inspect` reports
every axis including the ones that failed; `admit` returns `Ok` only when nothing refused and an `Err`
carrying **every** refusal otherwise (there is no `Established` to reach a report through); `census`
judges a set of rows cell by cell, and **an unchecked axis is never a pass**.

**The measured numbers** (from `clean-room/`, the commands at the end of this section):

| Claim | Measurement (printed) |
|---|---|
| An admitted document reports what it became | `admitted source.md as checksum_only — found 05285b942c550cd7a426d39f538f75dbc418d06f27c9b6b3efe085cfdfb46441, identity established = false`; the report carries all **6** axes, `5 of 6 axes established`, and the identity line reads *identity NOT established (no signer: a checksum is not a signature)* |
| The plan's own bar, §5 row 5 | `Established::ChecksumOnly.establishes_identity() == false` — **true**; the same document **pinned** to an operator's digest is `Established::Pinned`, `identity established = true, every axis established = true` |
| In place, never copied | one byte of the document changes and the **same row** refuses `source_digest_mismatch` with **both** digests — `362435c22bcc…62cc` found vs `05285b942c55…6441` declared; the row was never told to re-read anything |
| The undeclared and the unchecked paths, visibly distinct | one report holds **`carrier_undeclared`** (``the gate declares no carrier `U9`; it declares U3, U4, U5, U6, U7, U8``) **and** **`axis_unchecked`** (licence, acceptance) — **3** refusals, **2** names, and the axis states print `source active, licence unchecked, carrier refused, acceptance unchecked, identity inactive` |
| Every remaining name earned by the thing it describes | `axis_inactive` (a check declared **owed**), `licence_refused` (a refused reference carries nothing), `source_absent` (nothing to consume in place), `pin_mismatch` (the payload is not the release that was pinned) — all four `= true` |
| The census grades nothing it did not measure | **2** declared rows, **12** cells, `7 established / 2 inactive / 2 unchecked / 1 refused`, **admitted 1 of 2**: the row with an unchecked axis is not admitted |
| The refusal vocabulary is still closed | `RefusalKind::COUNT` is **57** and `ALL.len() == COUNT`; the seven new names are `ALL`'s, not a side list |
| No other system's number moved | every published digest reappears unchanged in the same run: U3 `460214b1…2fc6` (**4** lines, 1/2/4/8 workers), U4 `0e7358e4…4ed8` (**4**), U6's answer digest `25a485a5…` (**1**), U9 `32b3c1f8…` (**1**) and `c331cf55…` (**4**), the `465`-byte save |
| Gates | `cargo test --offline` **174 lib + 2 U4 + 6 U6 bridge + 5 U6 intake + 5 U6 transport + 4 U9 = 196 passed, 0 failed**; `cargo clippy --offline --all-targets -- -D warnings` exit 0; `cargo run --offline` exit 0, **byte-identical stdout across two runs** (**261** lines, `sha256 9ee6f8d7…`) — 240 lines before this pass plus exactly the **21** new `U6 intake:` lines |

**The four graded dimensions.**

| Dimension | Grade |
|---|---|
| **Correctness** — the verdict says what was actually proven | **graded and passed**: integrity is established by a digest read in place, identity is established **only** by pinning, and the two are different names on different types — `establishes_identity()` is `false` for a checksum and `true` for a pin, and every axis is reported whether it passed or not |
| **The named claim** — the gate's own vocabulary, neither more nor less | **graded and passed**: `admit`/`inspect → Established` (never a bool), the plan's census rule (*an unchecked axis is never a pass*), and refusals **by name** — `source_absent`, `source_digest_mismatch`, `carrier_undeclared`, `licence_refused`, `pin_mismatch`, `axis_unchecked`, `axis_inactive` |
| **The two refusal classes, kept apart** | **graded and passed**: a parse-level fault never existed here — this gate's split is *undeclared* (the gate declares no such carrier, no such verdict) versus *unchecked* (the gate declares the axis and nobody answered) versus *checked and refused*, and one report prints all three with their own names |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: the owed remainder is named in the module header, in the binary's own output and here; nothing was stubbed; `find crates -type f -newer clean-room/src/intake.rs` returns **0**, `cargo tree --offline` still shows `sha2` as the only dependency, and `clean-room` still declares its own workspace root |

```bash
cd clean-room
cargo test --offline                        # exit 0: 174 lib + 2 U4 + 6 U6 bridge + 5 U6 intake + 5 U6 transport + 4 U9 = 196 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 261 lines; identical across two runs (sha256 9ee6f8d7…)
cargo tree --offline                        # sha2 and its own tree; nothing else
find crates -type f -newer src/intake.rs    # 0 files: nothing under crates/ was touched
```

**Boundary check (P7), measured on the slice's own file:** no file under `crates/` is newer than
`clean-room/src/intake.rs` (returns **0**), `cargo tree --offline` still shows `sha2` as the crate's
only dependency, and the only occurrences of the string `crates/` in `clean-room/src/intake.rs` are the
witness lines this header carries (*"No `.rs` under `crates/voxel-package/` was read"*), never a `use`
or a `path`.

**What the gate still owes, and why each is not here.** Named, not half-built — a plausible verdict
from a stub would be exactly the invented default P2 forbids:

- **The document record and its compression** (`Compendium::derive`) — the documents *becoming* a
  recipe: it needs the record's own document table, which this slice does not declare.
- **The calibrated structural judge** (`StructuralJudge` + `calibrate`), the **ballot** and the
  **permission policy matrix** (`policy::decide`) — three vocabularies the plan names and the documents
  do not yet design here; writing them would be inventing vocabulary, and the census's *axes* are the
  only columns this slice declares.
- **The recipe/content format and the API document** (`apidoc::render`) — a format for content that
  U10's rows will carry and a published description of this table; neither is a judgement.
- **A row's wire form** — so a declared row can cross `transport::serve`; today a row exists only in
  process, which is why **the by-reference verbs (`path + digest`, never bytes) are still not rows of
  `bridge::TOOLS`** — the argument kind they need lands with the wire form.
- **A row's conformance against the world's own declarations** — `requires_stages` against U3's
  `Declared` and `requires_materials` against U2's `MaterialRegistry`, which is what `LEDGER` §55.3's
  *"conformance is against the world, not a list of strings"* asks for. It couples this gate to the
  owners' seams, so it is the next slice's first item rather than this one's last.
- **The crate × axis census over the tree** — twelve subjects of five axes. Its subjects are the tree
  itself, so it belongs with the Lane A/B measurement row (§5 row 8), not with intake.
- **U10's own tree of 91 declared rows** — the thing this gate exists to admit, built from
  `docs/MODS-MAP.md`; the gate now has the door, and the tree is U10's to declare.

**What did *not* change:** no existing answer, refusal, digest or measurement. The gate is a new module
(`intake`), seven new names in a closed vocabulary, five new unit tests, five end-to-end tests through
the public API (`tests/u6_intake.rs`), one new evidence function (`check_u6_intake`, called from
`check_u6`), U6's registry entry, and the *owed* line the binary prints — which now names the gate's
remainder instead of the gate itself.

---

## 19. U6's row by reference — a declared row admitted over the served line (2026-10-12, ninth pass)

**Why this, and why now.** §16 row 10 names **U10** (the mods-derived gameplay tree) and its hard
prerequisite in one sentence: *"deriving from a reference is an **admission**, never a transcription,
so `admit`/`inspect → Established` and the by-reference verbs must land first."* §18 landed the gate
itself — a declared row consumed **in place by reference** — but left the row reachable only from
inside the process: §18's own owed list says *"the by-reference verbs (`path + digest`, never bytes)
are still not rows of `bridge::TOOLS` — the argument kind they need lands with the wire form."* This
pass is that wire form, and nothing else: the table gains one row, the row arrives by reference over
the transport the previous passes built, and the two refusals a reference can produce are named with
their values and recorded like any other world-level refusal. No `.rs` under `crates/` was read (P7).

**What landed.** A row of the **closed table** now crosses the wire, and the gate consumes its
document **where it lives**:

- **`admit`, the sixth row** — eight declared arguments (`path`, `digest`, `feature`, `licence`,
  `claim`, `carrier`, `check`, `check_digest`), owner `intake.admit`, `changes_record: false`, and
  the checks `request_parsed + arguments_typed + owner_answered + recorded`. It declares **no**
  `claims_hold`, and says so by making no claim: the evidence an admission carries is the digest the
  intake read in place, and a file is not one of the owners a claim of this gate is re-checked
  against. A claim nothing can re-check is worse than no claim.
- **Three argument kinds with a shape rule** (`path`, `digest`, `name`): a reference is a **relative**
  path, refused by shape — leaving the root, a `..` segment, or any character the receipt's own
  fields separate with (`=`, `~`, `;`, `,`, `|`) — before any world is read, so a path that could be
  answered but not written back is refused `argument_malformed` and writes nothing.
- **The row dispatched to the intake's seam** (`intake.inspect` → `verdict`), the answer as
  `Answer::Admitted` with the pure digest a receipt needs and a wire form whose **axes travel by
  name** (`axes=feature:active|source:active|licence:active|carrier:active|acceptance:active|identity:inactive`),
  `identity` reported inactive because a checksum establishes integrity and never origin
  (`Claim::establishes_identity()` is `false`, and no `bool` carries it).
- **Two new names** in the closed vocabulary (`claim_undeclared`, and `RefusalSet::joined` so a
  channel that can carry one refusal carries every name in the set), and the two refusals a reference
  produces — `source_absent` with the path, `source_digest_mismatch` with **both digests** — passed
  through by name and **recorded**, because both lines parsed.
- **The root a reference resolves against**: the directory the server runs in (`.`) unless a caller
  pins one with `Gate::set_intake_root` — the definition of a relative path rather than a policy.
- **One defect found and fixed**: a receipt about an **empty** record wrote its absent head as `none`,
  which its own reader refuses (`malformed_wire`). Writer and reader now share one spelling for an
  absence (`-`), pinned by `every_answer_the_wire_writes_is_an_answer_the_wire_reads`.

**The four graded dimensions.**

| Dimension | Grade |
|---|---|
| **Correctness** — the row is read where it lives, and the answer is what it became | **graded and passed**: the document's bytes are hashed at the path the row cites and are never carried; the answer says what the row became (claim, found digest, every axis), the same row twice is one answer digest, and the served digest equals the one derived in-process |
| **The named claim** — a refusal by its own name, both values in the message | **graded and passed**: `source_absent` (the path named) and `source_digest_mismatch` (declared and found digest both in the message), kept apart from the parse-level `argument_malformed` a bad reference *shape* gets — and each recorded exactly once because the line parsed |
| **The two refusal classes, kept apart, over the pipe** | **graded and passed**: a served `admit` line that parses and cannot be honoured grows the record by one; a reference the table does not declare (`..`, an absolute path, a bad digest, a separator) is refused by the table and writes nothing |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: nothing stubbed, the remainder named in the module header, the binary's output, §19 and the four spec documents; `find crates -type f -newer clean-room/src/intake.rs` returns **0**, `cargo tree --offline` still shows `sha2` as the only dependency, and no file under `crates/` was read or touched |

```bash
cd clean-room
cargo test --offline                        # exit 0: 176 lib + 2 U4 + 4 U6 admit + 6 U6 bridge + 5 U6 intake + 5 U6 transport + 4 U9 = 202 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 270 lines; byte-identical across two runs (sha256 4f520076…)
cargo tree --offline                        # sha2 and its own tree; nothing else
find ../crates -type f -newer src/intake.rs  # 0 files (exit 0): nothing under crates/ was touched
```

**The stdout delta, accounted for line by line.** The baseline was **built**, not filtered: the same
sources with this pass's row and evidence removed, compiled cold (`RUSTC_WRAPPER=""`, a fresh target
dir) so no cached object could stand in for a source, printing **261** lines — the previous pass's
count. Against it the current run differs in **13** lines on one side and **4** on the other:

| Lines | Which | Why it must move |
|---|---|---|
| **9 inserted** | 2 for the new table row, 6 for its receipt (call, answer, checks, claims, effect, verification), 1 for the reference refusals | the row is new, so its declaration and its receipt are new |
| **3 changed values** | the table's own count (`5` → `6` declared verbs), the transport's vocabulary in the parse-fault line, and `U6 record: 4 → 6 events` with a new head | each prints a size or a record the change moves: this pass adds a row and records two refusals |
| **1 changed words** | the **U6 registry status** — six rows, `admit` arriving by reference, the wire form no longer owed | the status must say what is built; it is the first line a reader of `cargo run` trusts, and a status that still owed this would be a lie |
| **9 reworded in place** | the *owed* line, which no longer names the wire form | this pass lands it; the diff cannot show this one because both sides carry the new wording |

Every other line is byte-identical, **including every published digest of U1–U4/U6/U9** (U3
`460214b1…2fc6` ×4, U4's geometry digests, U6's five answer digests, U9 `c331cf55…` ×4 and the
`465`-byte save) — so nothing that was already true moved.

**One measurement had to be redone, and it settled an open thread.** The first baseline attempt built
the copy **outside the repository** — where the workspace's `rust-toolchain.toml` does not apply and
the default `stable-x86_64-pc-windows-**gnu**` host took over — and it disagreed with the tree in U3
and U9 (exit 1). That is not a cache artifact and not a race: each binary is stable across runs, and
a build with the rustc wrapper disabled reproduces the same numbers. Built with the workspace's own
pinned host (`stable-x86_64-pc-windows-msvc`), the *same copy* is **byte-identical** to the tree's
run. So the earlier veil over this ("a stale artifact, or something in a digest that depends on where
the tree is built") is lifted and answered: **the world digest depends on the target host, not on the
path, the working directory or the compiler cache.** The consequence is recorded here rather than
hidden: U3/U9's *"one digest at 1/2/4/8 workers"* bar holds on the host the toolchain file declares
(MSVC) and **fails on a GNU host**, where the 1-worker digest differs from the 2/4/8 one for the same
source. Why the GNU host differs is not settled here and is not this slice's to settle; what is
settled is that the bar is host-dependent, and that the tree's declared host is the one it was
measured on.
**What did *not* change:** no existing answer, refusal, digest or measurement. The five verbs this
spec first declared answer exactly as they did — their receipts, checks, claims and effects are
byte-identical in the diff above — and the record's discipline is untouched: a read appends nothing,
a world-level refusal is recorded by name, a line that does not parse writes nothing.

**What this still leaves owed** (named, unchanged from §18 unless marked): the document record and its
compression (`Compendium::derive`); the calibrated structural judge, the ballot and the permission
policy matrix; the recipe/content format and the API document; **a row's conformance against the
world's own declarations** (§18's other clause, still the next slice's first item); the crate × axis
census over the tree; U10's own tree of 91 declared rows; the bijection of the served table against
the legacy action vocabulary (a witness check P7 forbids reading the legacy for); `save`/`load` as
rows of the table; and a **pin** over a request line, which this table deliberately does not declare —
a pin names a signer and a digest an operator states out of band, and `claim_undeclared` says so.

## 20. U6's record made host-independent, and its receipts bound to the rows that made them (2026-10-12, tenth pass)

**Why this, and why now.** §19 landed the wire form and was then audited against the running binary. The
audit measured two defects *in the thing that had just landed*, both of them in the record's own currency:
a refusal's **record was not host-stable** (the payload carried the refusal's `detail`, and the intake put
this host's error text and this host's normalized path into that detail, so the same world refusing the
same request on two machines kept two different records), and a **receipt was not bound to its row** (the
reader anchored only the request to the table and then parsed the claims and checks lists independently, so
a receipt whose evidence was emptied was accepted and `verify` looped over an empty list and returned
`Ok`). This pass fixes both at the cause and pins each with a test that fails on the old behaviour. No new
capability: the table gains no row, no argument and no check, and the five older verbs' answers, the
repeat-is-one-digest rule and the parse-fault-versus-world-refusal split are untouched.

**What landed.**

- **The record carries the decision, not the explanation.** `refusal_recorded`'s payload is now
  `request=<canonical line> kind=<name>` — no `detail`. The chain hashes the payload into the record's own
  identity, and a detail is prose for the caller that may quote the host. Everything the refusal is
  *about* the request already carries; the words stay in the answer (`gate::Gate::record_refusal`).
- **The intake refuses in stable words.** A read that failed is described by the reference **as the row
  declared it** and the operating system's classification rendered in this crate's spelling
  (`no such document`, `permission denied`, `unreadable`) — never `std::io::Error`'s `Display`
  (`The system cannot find the file specified. (os error 2)`) and never the path this host normalized
  (`intake::read_reason`).
- **A receipt is bound to the row it names.** `Receipt::bind_to_row` (one place, asked by **both**
  readers) requires: the answer is the answer of the verb the request names; every claim is one the row's
  declaration allows; a row declaring `claims_hold` carries at least one claim; and the checks are exactly
  the declared ones in the declared order. `Receipt::from_wire` refuses a receipt that does not bind, and
  `Receipt::verify` refuses one built in-process that does not — `receipt_mismatch` (`RefusalKind` 59 →
  60), with the row's declaration and the receipt's own contents both in the message.
- **The claims each row may carry are now declared** (`bridge::table::Tool::claims`), which is what makes
  "bound to the row" checkable: `truth → truth_is`; `fields → field_is`; `geometry → truth_is +
  mesh_is_held + event_is`; `held → held_is`; `record → record_head_is`; `admit →` none. A test pins that
  the list is empty exactly where `claims_hold` is not declared, and `tests/u6_bridge.rs` checks the
  declaration from **both** sides on every row of the table (every claim a receipt carries is declared,
  and every declared claim appears in the receipt of a call that earned it). This is not the widening the
  mission forbids: nothing here expresses an **owed** check.
- **The NUL/CON asymmetry is folded into the shape rule.** Measured before this pass: `admit path=NUL` was
  **admitted** on this host (reading `NUL` yields no bytes, so the empty digest was what the row declared)
  while `path=CON` was refused `source_absent` — one class of reference getting two verdicts, decided by
  the operating system. A segment naming a reserved device (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`,
  `LPT1`–`LPT9`, with any extension) is now refused by the shape rule as `argument_malformed`, saying
  *names a reserved device, not a document*, and writing nothing (`0:-` in the record) — the same verdict
  on every host, because the rule is **declared** rather than probed. Measured after: `NUL` and `CON` are
  both `argument_malformed`; `null.txt`, `com10`, `lpt`, `aux_data`, `console.md` and `nulx` are still
  paths (`null.txt` refusing `source_absent` with the stable detail and being recorded).

**The four graded dimensions.**

| Dimension | Grade |
|---|---|
| **Correctness** — the record is a function of the decision, the receipt of its row | **graded and passed**: two differently-worded explanations of one failure now record **one** row (equal heads from two gates); an emptied claims or checks field is refused `receipt_mismatch` by both readers instead of verifying vacuously; a `held_is` claim in a `geometry` receipt is refused even though it holds in the same world |
| **The named claim** — a refusal by its own name | **graded and passed**: `receipt_mismatch` is the new name, and it names the row, the declaration and what the receipt carried (`geometry may claim truth_is, mesh_is_held, event_is and this receipt claims held_is`) |
| **The record's currency** — host-independence by construction | **graded and passed**: the payload is the table's canonical line and a closed vocabulary, so no detail — and therefore no host — can reach it; the served `source_absent` detail is stable across hosts (measured: `the declared reference `null.txt` could not be read in place: no such document`) |
| **Honesty and boundary** — what is owed is named | **graded and passed**: nothing stubbed; the remainder is named in the module header, the closure of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §20 and the five spec documents; `find crates -type f -newer src/systems.rs` returns **0** and `cargo tree --offline` still shows `sha2` alone |

```bash
cd clean-room
cargo test --offline                        # exit 0: 179 lib + 2 U4 + 4 U6 admit + 7 U6 bridge + 5 U6 intake + 5 U6 transport + 4 U9 = 206 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 270 lines; byte-identical across two runs (sha256 7ad33ae6…)
find ../crates -type f -newer src/systems.rs  # 0 files (exit 0): nothing under crates/ was touched
```

**The tests fail on the old behaviour, measured.** The three behaviours were reverted **in place** (payload
back to `detail=`, intake back to `{error}` + the host path, `bind_to_row` removed from both readers, the
device rule removed) and the suite ran red: **6** failures, exactly the five tests that pin these fixes
plus the one updated assertion — `a_refusal_is_recorded_by_the_decision_and_not_by_this_hosts_wording`,
`a_reference_that_names_a_device_is_refused_by_shape_on_every_host`,
`a_receipt_that_does_not_bind_to_its_row_is_refused_by_name`,
`a_receipt_that_lost_its_evidence_refuses_by_name_over_the_public_wire_form` (the public wire surface:
strip the receipt's claims or checks field, reload it, assert the refusal by name),
`the_bytes_are_read_where_they_live_and_never_carried`
and `a_parse_fault_writes_nothing_and_a_world_refusal_is_recorded_by_name`. The five edited sources were
then restored from byte-checked copies (`sha256sum -c` OK for all five) and the suite is green again.

**The stdout delta, accounted for line by line.** Against the pre-change baseline, `cargo run` differs in
**2** lines out of **270** and neither is a regression:

| Line | Old → new | Which old value was host-specific |
|---|---|---|
| the intake's `source_absent` detail | `C:\Users\ala\AppData\Local\Temp\…\gone.md could not be consumed in place: The system cannot find the file specified. (os error 2)` → ``the declared reference `gone.md` could not be read in place: no such document`` | **the old text was this host's**: its temp path and its operating system's sentence. The new text says the same thing on any host, still names the reference (`gone.md`), and still records the refusal |
| `U6 record: 6 events, head …` | `d65001c7…` → `8921b820…` | the record in that section ends with the recorded `geometry_out_of_residency` refusal, whose payload lost its `detail=` field — so the head hashes different bytes. The chain's length (6 events), the appended derivati

Every other line is byte-identical, including every published digest of U1–U4/U6/U9.

**What this still leaves owed** (named, amended from §19): **a row's conformance against the world's own declarations** (§18's clause, still the next slice's first item); the document record and its compression; the calibrated structural judge, the ballot and the permission policy matrix; the recipe/content format and the API document; the crate × axis census over the tree; U10's own tree of 91 declared rows; the bijection of the served table against the legacy action vocabulary; `save`/`load` as rows of the table; a **pin** over a request line, which the table deliberately does not declare, since a pin names a signer and a digest an operator states out of band (`claim_undeclared` says so); **the symlink question** — a reference that resolves inside the root through a link pointing elsewhere was not exercised (Git Bash's `ln -s` wrote a copy, not a link), so containment by link is unverified rather than claimed; the table print in `cargo run` spells each row's arguments, owner, record promise and checks but not yet its declared **claims** (the declaration is pinned by tests and named in refusals, but not printed); and a recorded refusal carries no subject beyond what the request line already holds — a deliberate narrowness, because the request is what the refusal is about.
---

## 21. U6's conformance axis — a row judged against the world's own declarations (2026-10-12, eleventh pass)

**Why this, and why now.** §18 and §19 each left the *same* clause first on their owed lists, and §16
row 10 waits on it as the last thing between the mods-derived tree (U10) and its prerequisite: *"a row's
conformance against the world's own declarations — `requires_stages` against U3's `Declared` and
`requires_materials` against U2's `MaterialRegistry`, which is what `LEDGER` §55.3's *'conformance is
against the world, not a list of strings'* asks for. It couples this gate to the owners' seams, so it is
the next slice's first item rather than this one's last."* This pass is that coupling and nothing else:
the vocabulary gains one axis and two refusals, a row carries one declaration about the world, and the
served table gains the two **optional** arguments a row uses to make it. No `.rs` under `crates/` was
read (P7).

**What changed, and why each change is the shape it is.**

- **The axis is appended, never inserted.** `Axis::Conformance` is the **seventh** column and
  `Axis::COUNT` is 7. A verdict's axes travel by position, so an axis inserted in the middle would
  silently relabel every report that already exists — and a report is evidence. The six axes the earlier
  slices declared keep their places and their answers.
- **Conformance is against the owners' seams, borrowed rather than copied.** `Intake::over(root,
  WorldDeclarations { roster, registry })` holds `&Declared` and `&MaterialRegistry`; there is no second
  roster and no second registry anywhere in the crate, which is this plan's rule 1 (*"the owners answer;
  the gate dispatches"*) applied to the one question that is *about* the owners. Two declarations are
  **equal by their digests** (`WorldDeclarations`' own `PartialEq`), never by pointer, so one registry
  borrowed twice is not two worlds. `Gate::new` now takes the registry beside the roster — a gate cannot
  be built without one, because a row that requires materials would otherwise have an axis nothing could
  consult — and `reference::registry()` is the process's **one** registry (a `OnceLock`), the same
  declaration the reference ladder's palette is drawn from. The gate is reached by the one path it was
  already reached by, and the two test helpers that assembled their own palette now call
  `reference::gate` — *less* code than before, and one world instead of three.
- **A requirement is a declaration, in one value.** `Requirement<T>` is `Named(items)` or `Owed` — the
  shape `Acceptance` already had for the bar the row *brings*, reused for the bar the world must
  *already have* — and `Requirements { stages, materials }` carries the two dimensions, because a
  refusal has to say **which** world did not answer. One value also keeps `Row::new` inside clippy's
  arity rule **without a suppression**: the constructor takes seven arguments and the row's tail is one
  named thing (*what this row requires of the world*) rather than two unnamed `None`s.
- **The axis distinguishes four facts, and only one of them is a pass.** A row that requires **nothing**
  is *reported* (`inactive`, subject `(none)`, **no refusal**) — the treatment a checksum-only claim's
  identity already gets, because *checked, and nothing to be there* is not a defect — a requirement the
  row declares and **owes** is `inactive` under `axis_inactive` (*"a bar that cannot be run cannot be
  met"*), one it declares and leaves **unnamed** is `unchecked` under `axis_unchecked` (*nothing was
  consulted*), and a named requirement the world does not hold is **refused with the missing item in the
  words**: `stage_undeclared` names the stage *and the roster's own stage names*, and
  `unadmitted_material` passes **the registry's own refusal** through — it is the owner that did not
  admit the id, and its message already names it. No state is read as a pass, which is §5 row 5's census
  rule holding on the new column.
- **The served spelling is two optional arguments, and the shape rule keeps them unambiguous.**
  `requires_stages` and `requires_materials` are the table's **first optional arguments** — absent means
  *this row requires nothing*, which is a declaration rather than a silence — of a new kind `Names`: one
  or more wire-shaped names joined by `+` (`+` is the list's own separator, not one the request or the
  receipt framing uses, so a list is one value and never a second field). A value that names nothing
  (`requires_stages=`) is refused **by shape** (`argument_malformed`: *nothing is not a value of a
  declared kind*, the way a zero-column patch is not), so it never reaches an axis and **writes
  nothing** — the parse/world split holding on the new arguments. A material that is not a `u16` is
  refused the same way a level that is not a number is; whether the registry *admits* an id stays the
  registry's refusal, not the table's.
- **The receipt wire form is `/2`, because an admission's axes are positional.** The tag exists to make a
  changed form a refusal rather than a misread: a `/1` reader would have read the new column as a claim's
  identity. The transport's own test now proves an *older* version is refused by name rather than
  half-read.
- **What the wire still cannot spell is named, not invented.** A requirement declared and **owed**, and
  one declared and **unnamed**, are reachable through the intake's seam (a caller building a `Row`) and
  are measured there — but not through a request line, for the same reason an owed *acceptance check* is
  unreachable while `check` is a required reference. The module header says so, `answer_admit` says so,
  the binary prints the owed line, and the seam's own tests measure the two states rather than pretending
  the wire carries them.

**The four graded dimensions.**

| Dimension | Grade |
|---|---|
| **Correctness** — an unchecked or unmet axis is never a pass | **graded and passed**: a conforming row's axis is `active` (stage `tectonics` from the served roster, material 1 from the served registry); a missing stage refuses `stage_undeclared` and a missing material `unadmitted_material`; requires-nothing and declared-and-owed are `inactive`, declared-and-unnamed is `unchecked`, and none of the four is a pass; the census's cells move 12 → **14** with no cell riding along |
| **The named claim** — a refusal by name, with the missing item | **graded and passed**: the missing stage is in the message *and* the roster's own names are (`the world declares no stage "no-such-stage" — it declares tectonics, …`), the missing material's refusal is the registry's **own** words with the id, and a key naming nothing is the table's own shape refusal; the detail is a bare negative nowhere |
| **The record's currency** — the world's declarations are borrowed, and equal by identity | **graded and passed**: `WorldDeclarations` compares by the owners' digests, the intake holds two borrows and copies neither vocabulary, `Gate::new` cannot be built without the registry, and `reference::registry()` is one value per process, shared by the binary, its evidence and every test |
| **Honesty and boundary** — what is owed is named | **graded and passed**: the two states the wire cannot spell are named in the module header, in `answer_admit`, in the binary's output and here; nothing is stubbed; `find ../crates -type f -newer src/intake.rs` returns **0** and `cargo tree --offline` still shows `sha2` alone |

```bash
cd clean-room
cargo test --offline                        # exit 0: 180 lib + 2 U4 + 5 U6 admit + 7 U6 bridge + 6 U6 intake + 5 U6 transport + 4 U9 = 209 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 276 lines; two runs byte-identical (sha256 d36cbffa…)
cargo tree --offline                        # sha2 and its own tree; nothing else
find ../crates -type f -newer src/intake.rs # 0 files: nothing under crates/ was touched
```

**The tests fail on the old behaviour, measured.** The axis's judging body was reverted **in place** to
exactly what it answered before this pass (reported, never judged) and the suite ran red with
`--no-fail-fast`: **3** failures — `conformance_is_decided_against_the_worlds_own_declarations` (the
seam), `a_served_row_is_conformed_against_the_worlds_own_declarations` (the **served binary**, a separate
process over a pipe) and `a_refused_row_is_not_partially_admitted_and_every_failing_axis_reports` (whose
all-axes-fail row now carries the conformance refusal) — while the two table tests stayed green, because
the *arguments* are a different behaviour from the *judging*. The file was then restored from a
byte-checked copy (`sha256sum`: `1732ae31…` before and after) and the suite is green again.

**The stdout delta, accounted for line by line.** Against the previous pass's final run (270 lines,
sha256 `7ad33ae6…`), `cargo run` differs in **13** lines — **7 changed, 6 added, none removed**, and
**every one of them inside U6's own two blocks** (the registry entry at lines 41–47 and U6's evidence at
167–257); U1, U2, U3, U4, U9 and the **five older verbs' answers** are byte-identical, and so is every
published digest except the admission's own answer digest (which *must* change: the answer is what the
row became, and the row is now judged on a seventh axis).

| line | old → new | why it is not a regression |
|---|---|---|
| `[U6] …` registry status | *"judged on named axes"* → *"judged on **seven** named axes … the last axis is **conformance against the world's own declarations** …"* | a status is a claim about what is built, and the built thing gained the coupling §18 named owed |
| `interface: intake::{…}` | `Intake::admit/inspect` → `Requirement, Requirements, WorldDeclarations, Intake::over/admit/inspect` | the seam's own declaration — the line the crate's registry entry has always carried |
| `U6 admit path:path, …` | 8 arguments → 10, the last two `requires_stages:list of names, requires_materials:list of names` | the table print is the table; the two new keys are the pass |
| `U6 answer a-declared-row … axes …` | `… identity=inactive` → `… identity=inactive conformance=inactive`, digest `5de235e3…` → `f489311f…` | the answer is the report of what the row became, axis by axis; a comparison against the world that *was* made cannot be unsaid, and the digest over it moves with it |
| `U6 intake: pinned by an operator …` | *"every axis established = true"* → *"every **demanded** axis established = true, reported rather than counted = [\"conformance\"]"* | the old sentence would now be **false**: this row requires nothing, and calling that a pass is the invented default P2 forbids |
| `U6 intake: states — …` | five states → six (`conformance inactive`) | one more column, reported where it is not a pass |
| `U6 intake: census: 2 declared row(s) …` | `12 cells, 7 established / 2 inactive / 2 unchecked / 1 refused` → `14 cells, 7 established / 4 inactive / 2 unchecked / 1 refused` | two more cells, both `inactive` (reported), and the admitted count is still 1 of 2 |
| *(added)* `U6 intake:   conformance inactive — … ((none))` | — | the pinned row's own finding, printed like every other finding |
| *(added)* `U6 intake: conformance against the world: …` | — | the measurement this pass exists for: a stage the roster runs and a material the registry admits = `active`; a stage nobody declares = `stage_undeclared`; a material the registry never admitted = `unadmitted_material` |
| *(added)* `U6 intake: conformance states: requires nothing = inactive, declared and owed = inactive, declared and unnamed = unchecked (distinct = true)` | — | the three non-pass states, measured distinct |
| *(added)* `U6 intake: conformance still OWED over a served line — …` | — | **the owed line**: what the wire cannot spell, and where it *is* reachable |
| *(added, ×2)* `U6 intake:   [a complete row] / [a declared row] conformance inactive — …` | — | the census prints every cell that is not a pass, which is exactly what makes the new column visible in a grid |

**What did *not* change:** the five older verbs' requests, answers, claims, checks and effects; the
admission's own eight arguments and the shape of its answer; every other digest the run publishes; the
crate's dependencies (`sha2` alone); and nothing under `crates/`.

**What this still leaves owed** (named, amended from §20): the **served spelling of a requirement that is
owed or unnamed** (the axis has the two states; the wire refuses them by shape rather than inventing a
spelling — the one clause of §55.3 this pass does not close); the document record and its compression;
the calibrated structural judge, the ballot and the permission policy matrix; the recipe/content format
and the API document; the crate × axis census over the tree; U10's own tree of 91 declared rows; the
bijection of the served table against the legacy action vocabulary; `save`/`load` as rows of the table; a
**pin** over a request line, which the table deliberately does not declare, since a pin names a signer
and a digest an operator states out of band (`claim_undeclared` says so); **the symlink question** — a
reference that resolves inside the root through a link pointing elsewhere was not exercised (Git Bash's
`ln -s` wrote a copy, not a link), so containment by link is unverified rather than claimed; the table
print in `cargo run` spells each row's arguments, owner, record promise and checks but not yet its
declared **claims**, and now not yet which arguments are **optional**; and a recorded refusal carries no
subject beyond what the request line already holds — a deliberate narrowness, because the request is what
the refusal is about.

## 22. U6's receipts re-established — an admission that makes no claim is re-derived, not believed (2026-10-12, twelfth pass)

**Why this, and why now.** §20 bound a receipt to its row's **declaration** (the answer is that verb's
answer, the claims are the row's, the checks are exactly the row's) — and the read-only audit that
followed §21 measured what that binding does *not* cover. The `admit` row is the one row that makes no
claim on purpose, so its receipt carries no evidence a claim-check could re-run: its two references, its
requirements and its answer's axis states are all **carried values**. Editing any of them left
`Receipt::from_wire` and `Receipt::verify` both returning `Ok` — a receipt could be doctored into
evidence of an admission that never happened (a stage the roster does not declare, a material the
registry never admitted, a document moved or zeroed, a flipped `conformance` or `identity` state) — while
the same edit to a *structural* column (the tag, the axis nouns, the checks, the evidence columns) was
already refused. This pass closes that gap and nothing else: no axis, no argument and no answer changes;
verification gains a way to re-establish an admission from its request. No `.rs` under `crates/` was read
(P7).

**What changed, and why each change is the shape it is.**

- **One place decides what a request means.** `Request::row` builds the row an `admit` line declares
  (`specs/005-u6-record-api/contracts/u6-seams.md` C3's row, argument by argument), and
  `receipt::admitted_answer` asks the intake what that row becomes. `Gate::answer_admit` now calls those
  two — so the row a call is dispatched with and the row a receipt is re-checked against are built by the
  **same code**, and cannot drift into two similar admissions. Nothing about the answer moved: the same
  seam, the same axes, the same digest.
- **The verification context carries the content gate's door.** `Owners` gains `intake: &Intake` —
  **required**, not optional, because an admission receipt verified without a door would have nothing to
  re-read, and a missing declaration has to refuse rather than pass. `Gate::owners` fills it from the
  gate's own intake, and `Gate::intake` exposes it for the hand-built owners the tests and the evidence
  use for a moved world or a repaired chain.
- **An admission is re-established, not believed.** `Receipt::verify` asks the seam again for an
  admission: the request is read as the row it declares, the intake answers what that row is *now*, and
  three kinds of edit are caught. (i) A request the world cannot decide refuses by the **owner's own
  name** — `source_absent`, `source_digest_mismatch`, `stage_undeclared`, `unadmitted_material`, the
  table's `argument_malformed` for an id that is not an id — which is also what happens when the document
  or the cited declaration is not reachable from the verifier's own door. (ii) A carried fact the request
  and the world do not support — the feature, the claim, the document digest, or any axis's state —
  refuses `receipt_mismatch` **naming the fact and both values** (the request and the world establish
  `conformance=active` and this receipt carries `conformance=inactive`). (iii) An effect an admission
  cannot have had — an appended event, a record that moved, geometry that changed, work that was done —
  refuses `receipt_mismatch` too; its *absolute* positions are the call's rather than the receipt's (the
  record has moved since), so they are compared with themselves rather than re-derived.
- **Everything that was protected stays protected, and the older rows are untouched.** `bind_to_row`
  still refuses a receipt whose answer is another verb's, whose claims are another row's or emptied,
  whose checks were reordered or dropped; the tag still makes `/1` a refusal rather than a misread; the
  five older rows still verify by their claims, and their answers and digests are byte-identical. An
  admission appends nothing, so the record's host-stability is untouched: two hosts with different
  working directories still produce one head and one refusal detail per line.
- **What a faithful re-derivation would need, and is therefore named rather than invented.** The five
  older rows' answers are **not** re-established from their requests: their claims *are* their evidence,
  and re-deriving `fields` or `geometry` would perform work and move the owners (U4's ledger, the
  resident set, U2's record) — a read-only re-evaluation seam neither `spec.md` nor the LEDGER designs.
  It is named in `src/bridge/mod.rs`, in `src/bridge/receipt.rs`, in the binary's output and here. One
  consequence is stated rather than hidden: for those rows an answer field edited to contradict the claim
  beside it is still not refused by that comparison.

**The four graded dimensions.**

| Dimension | Grade |
|---|---|
| **Correctness** — an admission is what its request and the world establish | **graded and passed**: an honest served admission verifies and equals the answer derived here; a stage nobody declares refuses `stage_undeclared`, a material never admitted `unadmitted_material`, an id that is not one `argument_malformed`, the requirements stripped and a flipped axis state `receipt_mismatch`, a moved document `source_absent`, a zeroed digest `source_digest_mismatch`, an impossible effect `receipt_mismatch` — and a verifier whose tree holds no such document refuses `source_absent` instead of passing |
| **The named claim** — the refusal is the owner's own, with its values | **graded and passed**: every refusal is passed through by the seam's own name with the fact and both values in the message (no host path, no paraphrase, no bare negative), and a re-establishment that contradicts the receipt is refused `receipt_mismatch` — the name §20 gave to a receipt that is not its row's |
| **The record and the five older rows** — nothing moves | **graded and passed**: `cargo run` differs from the previous accepted run in **2 added lines and nothing else** (`215a216,217`), so the five older verbs' answers, U1–U4 and U9 are byte-identical; the record still grows only where a change happened, and verification writes nothing at all; a doctored receipt refuses while an honest one verifies |
| **Honesty and boundary** — what is owed is named | **graded and passed**: the un-re-derived rows and the read-only seam they would need are named in the module header, in the receipt's own documentation, in the binary's output and here; `find ../crates -type f -newer src/evidence.rs` returns **0**; `cargo tree --offline` still shows `sha2` alone; and this pass deliberately does **not** correct the three documentation inconsistencies the audit listed (the stale owed line, the `/2` rationale, `tasks.md` T044's numbers) — they are left exactly as they are |

```bash
cd clean-room
cargo test --offline                        # exit 0: 180 lib + 2 U4 + 5 U6 admit + 7 U6 bridge + 6 U6 intake + 6 U6 transport + 4 U9 = 210 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline                         # exit 0; 278 lines; two runs byte-identical (sha256 3328d3c9…)
cargo tree --offline                        # sha2 and its own tree; nothing else
find ../crates -type f -newer src/evidence.rs # 0 files: nothing under crates/ was touched
```

**The tests fail on the old behaviour, measured.** The re-establishment was removed **in place** (in a
byte-identical copy of the tree, `Receipt::verify` back to trusting the carried answer) and the new test
ran red: `a_doctored_admission_refuses_by_name_while_an_honest_one_verifies` fails at its first doctored
case — *"a stage the roster does not declare: a doctored admission must refuse rather than verify"* —
while the other five transport tests, which drive the same binary as a separate process, stay green. The
new test is therefore red on the older gate and green on this one.

**The stdout delta, accounted for line by line.** Against the previous accepted run (276 lines, sha256
`d36cbffa…`), `cargo run` differs in **one hunk, `215a216,217`** — **2 lines added, 0 changed, 0
removed** — both inside U6's own evidence block:

| line | what it says |
|---|---|
| *(added)* `U6 receipts: an admission re-established from its request — an honest one verifies = true; a stage nobody declares [stage_undeclared]; the requirements stripped [receipt_mismatch]; a carried axis flipped [receipt_mismatch]; a verifier whose tree holds no such document [source_absent] (every one as declared: true)` | the measurement this pass exists for: the receipt is doctored four ways through `Receipt::from_wire` and `verify`, exactly as a client would, and the honest one still verifies |
| *(added)* `U6 receipts still OWED: the five older rows' answers are not re-established from their requests …` | **the owed line**: which rows are not re-derived, and the read-only seam that would be needed |

**What did *not* change:** `TOOLS`, every argument and its kind, every answer and its digest, the claims,
the checks, the effect of every call, the record's payloads, the wire form and its version, the five older
rows' verification, and nothing under `crates/`.

**What this still leaves owed** (named, amended from §21): the **five older rows' answers** are not
re-established from their requests, for want of a read-only re-evaluation seam the spec and the LEDGER do
not design — so their answers are only as bound as their claims make them; the served spelling of a
requirement that is **owed or unnamed**; the document record and its compression; the calibrated
structural judge, the ballot and the permission policy matrix; the recipe/content format and the API
document; the crate × axis census over the tree; U10's own tree of 91 declared rows; the bijection of the
served table against the legacy action vocabulary; `save`/`load` as rows of the table; a **pin** over a
request line; the symlink question; and the table print's missing claims and optional markers.
---

## 23. U10's first slice — the tree of the record's rows, and the four parts it owes (2026-10-12, thirteenth pass)

**Why this, and why now.** §16 row 10 names **U10** (the mods-derived gameplay tree) and its hard
prerequisite: *"deriving from a reference is an **admission**, never a transcription, so `admit`/`inspect
→ Established` and the by-reference verbs must land first."* Those landed in §18 (§19 served the row,
§21 added conformance, §22 made an admission's receipt re-established rather than believed), so the
prerequisite is met and the tree can be built as what the record says it is: **a list of U6's own rows**.
This pass builds the first coherent slice of it and nothing else: **one entry per reference** in
`docs/MODS-MAP.md` §1–§5, the record's own four-word disposition for each, and **one row per technology**
under the record's double-cover rule. No `.rs` under `crates/` was read (P7), **no mod source was read at
all** (the harvest is the reading — `docs/MODS-MATERIALS.md` §7 records the sweep complete and every clone
HEAD-pinned), and **no dependency was added**: `sha2` stays the crate's only one.

**What landed.** `clean-room/src/gameplay.rs` is one module and one capability: *the tree*, as two lists and
their link.

- **The disposition vocabulary** — the record's own four words (`mapped`, `open`, `refused`, `unresolved`,
  from `MODS-MAP` §1's legend and §3/§4's *"Unresolved — recorded, not guessed"*), each with its `name`, its
  `parse` and a `COUNT` pinned to `ALL.len()`, and each mapped to the gate's own `Establishment` (`mapped` is
  the only pass, `refused` is `Refused`, an owed one is `Inactive`). It is the **tree's** field, not an eighth
  axis: an axis is a question about **a row**, and an `open` reference has no row for an axis to judge (spec
  D5). It lives in the tree's module rather than in `intake.rs` (where `tasks.md` T001 named it) so U6's axes,
  verbs and answers stay untouched — a deviation named in the task file.
- **The entry** — `Entry { reference, disposition, row, note }`, with the shape rules that make a *tree* of it:
  a `mapped`/`refused` entry declares a row and an `open`/`unresolved` one declares **none**, and the two ways
  of getting that wrong are refused — `argument_absent` for the missing declaration (there is no default) and
  `argument_malformed` for the row that is not of its declared kind. **Both are the closest *declared* names,
  and the exact word the record would give the fact is printed as owed** rather than coined
  (`specs/007-u10-gameplay-tree/contracts/u10-seams.md` C1).
- **The tree** — 106 entries in the record's order (10 in §1, 42 in §2, 19 in §3, 35 in §4; the map's §5 list
  and its repeated mods fold to one entry each) and 43 technology rows. `Tree::declare` is the double-cover
  rule as code: a descendant whose row **is** its ancestor's row is one citation on one row, and an entry
  bringing a **different** row for a held technology is refused `duplicate_output`, with
  `docs/MODS-MATERIALS.md` §8.2's rule quoted and the **name it is owed** printed in the refusal itself.

**The readings this pass took, named rather than hidden** (the module header carries all three, and each is
checkable against the record):

1. **A disposition is the record's own word.** `mapped` where the record states a feature; `open` where it
   assigns none (the map's `*`, §3's *"N/A until multiplayer"*, and the §2 references the harvest names in no
   rule and describes no process for — `optical`'s empty clone, `create-new-age`'s *"scope = energy, nothing
   further"*); `refused` where a licence fact forbids derivation (the map's two, plus `MFR`'s *"no licence file
   anywhere in the tree"* and `Create-The_Factory_Must_Grow`, *"not cloned and not read"*); `unresolved` where
   the record's own destination says so.
2. **A reference's technology** is the one the record states for it: the process its harvest passage states,
   else the first §8.1 rule row that cites it, else its own class row in §3/§4 — and where none of the three
   names one, the record assigned no feature and the entry is `open`.
3. **Two lists, kept apart.** A row's `cited_by` is the record's own list of who proposes the rule (§8.1,
   where the double-cover proof is visible); the entries whose technology *is* that row are what its licence
   verdict is read from. A permitted row that cites a no-derivation reference keeps its verdict **and prints
   the refused citation** (`refused_citations`), because a verdict is per reference and averaging two verdicts
   into silence is exactly what this repository refuses.

**The measured numbers, read back out of the systems they belong to** (`cargo run --offline`, exit 0):

| Printed | Value |
|---|---|
| `U10 tree` | **106 entries** (one per reference, the record's order) and **43 rows** (one per technology) |
| `U10 census` | `entries=106 mapped=64 open=25 refused=5 unresolved=12` — the four summing to 106 — beside the record's own **106 references (10 §1 + 42 §2 + 19 §3 + 35 §4), the 91-URL file the map was written from, this repo's 97-URL copy, 24 `open` (§2) **[since corrected to nine — see the §2-legend correction below]** and 43 `open` rows (prior disposition file)**, un-averaged |
| `U10 rows` | 43 technologies; **23 carry more than one citation**; **35 declare their own bar owed** |
| `U10 gate` (U6's own census) | 43 rows over 7 axes (**301 cells**): **177 established, 121 inactive, 0 unchecked, 3 refused — admitted 8 of 43** |
| `U10 refusals by name` | `licence_refused×3`, `axis_inactive×35` |
| `U10 owed references` | **37 of 106** entries are named owed and carry no row (never admitted) |
| `cargo test --offline` | **226 passed, 0 failed** (was 210: +9 lib, +7 `u10_gameplay`; the seventh is the SC-001 check below) |
| clippy · `find ../crates -newer …` | exit 0, no diagnostics · **0 files** |

**Corrected after the pass (2026-10-12, user).** Rows of the record were wrong and are carried
corrected in `docs/MODS-MAP.md`'s header and in the entries: `tt20` is a **performance mod** (a TPS
fixer), not the network/tick budget; `resquake` is the **Squake movement model** — with
`Minecraft-CameraOverhaul` added to the game's player controller (`docs/LEDGER.md` C-150), **not in the
clean room** — so its entry leaves `unresolved` for `open` with no row assigned; and the Archipelago row is
**re-keyed** to the artifact the reference (`docs/references.md`) points at — the Modrinth **datapack**
`archipelago`, world generation (island/archipelago terrain) — whose own terms declare `CC-BY-NC-ND-4.0`
(non-commercial, **no derivatives**), so that entry moves from `open` to **`refused`** and
`MinecraftFabricArchipelagoMod` is demoted to the replaced URL it is (the randomiser client, a different
artifact). That correction left the table reading `open=24`, `refused=5`, `unresolved=12`, with `23`
multi-cited rows and `36 of 103` owed — the counts the **second** correction below then moved.

**The eleven stdout lines that moved, named** (old → new; the line count stays **441**): `U10 documents`'s
map digest `d1de98e5…` → `104113e0…`; `U10 census` `open=25 refused=4` → `open=24 refused=5`; the entry line
`MinecraftFabricArchipelagoMod open (…)` → `archipelago refused feature=world-shape licence=refused …`;
`U10 rows` `22` → `23` citations; the `world-shape` row's `cited_by`/`refused_citations` gain `archipelago`;
`U10 bar` `66` → `67 row(s) declared`; the head's full map digest; `U10 owed references` `37` → `36 of 103`;
`U10 in place`'s tampered digest `e0794314…` → `e77ffbff…` and its declared `d1de98e5…` → `104113e0…`; the
double-cover line's `22` → `23 row(s)` and its witnesses list, which now carries
`world-shape=BigGlobe+archipelago`. Nothing else in the 441 lines differs.

**Corrected again (2026-10-12, user) — the three references that had no entry, given one.** `TerraFirmaCraft`
and `Calculator` are in `docs/references.md` and are cited all through `docs/MODS-MATERIALS.md` §8.1, but they
were never in `docs/MODS-MAP.md` §2's slot table and so had no entry at all; `Minecraft-CameraOverhaul` had
neither an entry nor a mention of its own — its only trace was inside the corrected `resquake` note — although
`docs/LEDGER.md` C-150 names it as part of the game's player controller. Each now has an entry, and each
entry's evidence is stated in the map's own row for it, read from the artifact's own source rather than from a
summary: **`TerraFirmaCraft` → `mapped`, `smelting`** (clone `6e77733c` of
`github.com/TerraFirmaCraft/TerraFirmaCraft`; `LICENSE.txt` = **EUPL-1.2**, the European Union Public Licence
v1.2 — copyleft, derivation permitted, nothing copied — with the provenance in `docs/MODS-MATERIALS.md` §2:
20 rocks each with a real geological class, 32 ores under a grade system, 27 metals and the carburisation
steel chain, 9 alloy recipes carrying exact composition ranges, the colour-temperature forge ladder in
`Heat.java`, and the processing chain as data); **`Calculator` → `mapped`, `smelting`** (clone `c796f1dc` of
`github.com/SonarSonic/Calculator`; `LICENSE` = **MIT**, © 2018 Ollie Lansdell; circuits, purified and
enriched coal, coal dust, reinforced iron ingot); and **`Minecraft-CameraOverhaul` → `open`** (its own
repository's description and `LICENSE.md` = **GPL-3.0** — adopted into the game's player controller with
`resquake`, **not in the clean room**, so the record assigns it no row, exactly as `resquake` has none).
**The smaller reading, taken and said:** each new `mapped` entry is filed under its reference's **first** §8.1
rule (furnace firing / smelting, the rule citing `TFC heating (883; 930 °C sample) · Calculator
ReinforcedFurnace`), because the harvest's measured passage states a whole processing chain for a reference
while the rule library states rules one at a time — a feature is a rule. No new row, axis or verb: `smelting`
keeps its five citations, the row count stays **43**, and U6's census over the rows and the four owed rulings
are untouched. Counts: entries **103 → 106**, `mapped` **62 → 64**, `open` **24 → 25**, owed **36 of 103 →
37 of 106**; `cargo test --offline` is unchanged at **225 passed / 0 failed** (nothing new is tested — the
existing test's pinned counts move), clippy stays clean, and `find ../crates -newer src/gameplay.rs` still
returns nothing.

**The ten stdout lines that moved, and the three that were added** (old → new; the line count goes **441 →
444**): `U10 tree` `103` → `106 entries`; `U10 documents`'s map digest `104113e0…` → `b1b9b2f2…`; `U10 census`
`entries=103 mapped=62 open=24` → `entries=106 mapped=64 open=25` (and the sum beside it); **+** the entry
line `TerraFirmaCraft mapped feature=smelting licence=permitted carrier=U3 source=docs/MODS-MATERIALS.md`…;
**+** the same line for `Calculator`; **+** `Minecraft-CameraOverhaul open (owed: camera tilting and roll for
the player controller …)`; `U10 bar` `67 row(s) declared by 103 entr(ies)` → `69 … by 106 entr(ies)`; the
head's full map digest; `U10 owed references` `36 of 103` → `37 of 106`; and `U10 in place`'s tampered digest
`e77ffbff…` → `ac2bada3…` together with its declared `104113e0…` → `b1b9b2f2…`. Nothing else in the 444 lines
differs: `U10 rows`, all 43 `U10 row:` lines, `U10 gate` and the two double-cover lines are byte-identical,
including every published digest of U1–U4/U6/U9.

**What the correction is read from, named** — `docs/MODS-MAP.md`'s dated header note and its two dated rows in
§2 plus the §4 row beside `resquake`; the module header's own paragraph (`clean-room/src/gameplay.rs`); the
licence facts above from each artifact's own licence file (`LICENSE.txt`, `LICENSE`, `LICENSE.md`); the clone
pins from `F:/shploingus-mod-sources/_harvest/clone-hashes.txt`; and the harvest's own passage and rule rows
(`docs/MODS-MATERIALS.md` §2, §8.1).

**The refusals are exercised, not asserted.** The gate's own name `licence_refused` is raised by the five
refused references and by nothing else; the document is read **in place**, so an absent one refuses
`source_absent` with its path named and one whose bytes have moved refuses `source_digest_mismatch` **with both
digests in the message** (measured over a tree of the evidence's own, with only the bytes moving); a licence
word outside the two refuses `licence_undeclared` **with the declared two in the message**; and the tree's rows
cross the **served** wire form — one `admit` line through the real binary as a separate process, with the seven
axes in the declared order (`identity` and `conformance` reported, never counted as passes) and the answer
digest equal to the one derived here over the same line.

**The stdout delta, accounted line by line** (stdout 278 → 441 lines for the build pass; the two 2026-10-12
corrections then added **+3** entry lines and moved ten — 441 → **444** — with nothing removed):

| Lines | What they are |
|---|---|
| **+103** | `U10 entry:` — one line per reference, which is US1's own independent test (*"print one line per entry — its reference id, its disposition, and (for a row) its feature, source, licence, carrier and check"*); **+3** more entry lines, one per reference the two corrections supplied — `TerraFirmaCraft`, `Calculator`, `Minecraft-CameraOverhaul` |
| **+43** | `U10 row:` — one line per technology, naming its carrier, its verdict, its check and the record's citations |
| **+17** | the tree, documents, census, rows, gate, refusals, the three `bar` lines, owed-references, in-place, the two double-cover lines, and the **four `U10 OWED:` lines** |
| **2 changed** | the registry's `[U10]` status line and its `interface` line (`src/systems.rs`) — *"absent"* is no longer the honest status |
| **0 removed** | every other line is byte-identical, **including every published digest of U1–U4/U6/U9** |

**What this still leaves owed** (named in four places — here, `specs/007-u10-gameplay-tree/plan.md`,
`research.md` and the binary's own output — so a reader meets each of them four times rather than not at all):

1. **The recipe document's row-kind key and version.** A gameplay entry is a new row kind in a format `LEDGER`
   **R-64.1** makes *versioned and closed* (`packages/alpha-shell.shspec` declares `window`, `tab`, `bar`,
   `filed`, `pin`, and the compiled ticket's `row`). No reference chooses the key or the number, so the tree
   cites the existing form and prints the gap.
2. **The name of the refusal for a second row on a descendant technology.** §8.2 states the **rule** and no
   reference states the **word**; the tree refuses with the closest declared name (`duplicate_output`) and
   prints that the exact name is owed.
3. **A verb that serves the whole tree.** U6's table is closed and bijective and no design declares one; the
   tree's **rows** cross today over `admit`, the **list** does not. What the tree holds is reported in-process.
4. **The per-row family assignment** (the ten families) and the `SUBSTANCES` table shape — `MODS-MATERIALS`
   §5/§8.3 leave them owed, and §8.3 calls its own progression model *"a design sketch, falsifiable"*. This pass
   names the technologies and assigns **no** family.

**One disagreement the record itself does not resolve, reported rather than averaged.** The map's §2 `*`
marks 9 of the "24 `open`" mods while the harvest names the rest of them in its rule library, so the count
this build derives (**25** after the second correction, which added one `open` and two `mapped` entries)is printed beside both the §2 figure and the prior file's 43. *(Corrected 2026-10-12: the 24 was not reproducible from any source — see the §2-legend correction below.)*

**SC-001 settled — and the header, not the tree, was the wrong part (2026-10-12, user).** The criterion had
been reported unmet — *the tree's entry count equals `MODS-MAP`'s own reference count* — with the header
declaring **91 URLs** against the tree's **106** and declining to say which was wrong. Counted from the
sources, the two figures describe **two different documents**, and the map's own rows are a third set that is
larger than both.

| Source | Counted 2026-10-12 | What it is |
|---|---|---|
| `references/shploingus_dirty/references.md` (sibling `shploingus-game` tree) | **91 URL lines**, 91 slugs | the file the map was written from — the figure the header has always quoted, correct **for that file** |
| `design/mod-dispositions.jsonl`, beside it | **83 rows** (**40 `mapped`**, **43 `open`**) | the prior work the header cites, whose 8 row-less URLs are §5 |
| `docs/references.md` (this repo) | **97 URL lines**, 97 slugs | that file **minus three and plus nine** |
| `docs/MODS-MAP.md` §1–§5, the map's own rows | **106 references** (10 §1 + 42 §2 + 19 §3 + 35 §4) | the count the tree derives — **the equality SC-001 asks for** |

The reconciliation, each step measured: the copy = the dirty file − `C2ME`, `CubicChunks`,
`MinecraftFabricArchipelagoMod` (the replaced URL) + nine (`TerraFirmaCraft`, `Calculator`,
`Minecraft-CameraOverhaul`, `archipelago`, `immersivethunder`, `sound-physics-remastered`, `mc-ar-mod`, and the
two links that are **not mods** — the `VictorTaelin` gist and the `blackflow.co.uk` article). **94** of the
copy's 97 URLs correspond to a row — the **88** both files carry, plus the **6** the copy adds that this map
gives rows — with **3** spelling variants folding onto one row each (`chunky` → `MOD-chunky`, `tidal` →
`MOD-tidal`, `stockbridge(refinedstrotage%2Bcreate)` → `stockbridge`). The **12** rows the copy does not carry
are `C2ME` and `CubicChunks` (dropped from the copy, kept here) and the **10 Tekkit mods** the dated
2026-10-03 ruling added. **94 + 12 = 106** — the map's own count, and the tree's entry count. **So the
header was corrected, not the entry set**, and SC-001 is now **measured true**: the header states
`**The map's own reference count: 106 references**` with the split and the arithmetic, the census line prints
that count against the derived entry count, and `the_maps_own_reference_count_is_the_trees_entry_count`
re-derives it from the document **in place** — folding the spelling variants the map prints and comparing the
sets **both ways**, so a dropped row or an unnamed entry fails by name. Nothing was invented or dropped to
make the numbers meet: `mc-ar-mod`, the one mod URL in the copy that no section of the map names, is named
**owed** in the map's §6 and derives no entry.

**The five stdout lines that moved, and the none that was added** (444 → **444**): the `[U10]` registry
witness (`91 URLs` → `**106 references** … its source file's 91 URLs and this repo's copy's 97`);
`U10 documents`'s map digest `b1b9b2f2…` → `48e37378…`; `U10 census`'s record-figures tail (the header's 91
URLs → the three reconciled counts, with the map's own 106 set against the derived entry count); `U10 bar`'s head digest, likewise; and `U10 in place`'s tampered digest `ac2bada3…` → `70abbf06…`
with its declared `b1b9b2f2…` → `48e37378…`. Every other line is byte-identical, `U10 tree`, all 43
`U10 row:` lines, `U10 rows`, `U10 gate` and `U10 owed references` included, and the test suite grew by
exactly the one check (225 → **226**).

**The map's §2 `*` count, corrected — and the split guarded (2026-10-12, user).** The audit found the map's
§2 legend standing on a figure no source reproduces — it said `24 of them` against **nine** `*` marks in its
own rows — while the tree derives **twelve** §2 `open` entries (the nine, plus `optical`, `create-new-age`
and `EE 3`, each for the reason its own entry states) and the prior disposition file's own figure is the
**43 `open`** rows this section prints beside it. The legend was corrected to what the sources support (the
nine marks, the three named further `open`s, twelve in all, dated in place); the census's §2 figure moves
**24 → 9**; §5's heading records that one of its eight slots was re-keyed; and the README line now describes
the map's own scope rather than the copy alone. The SC-001 check — which until now guarded only the total —
also re-derives the header's **per-section split** (`10 §1 + 42 §2 + 19 §3 + 35 §4`) from the rows **in
place**, and was proved to bite by moving §3's `shuttfup`/`Stfu` row into §4: the check failed by name
(`[10, 42, 17, 37]` against the stated split), and the map was restored byte-exact. **The stdout delta is
four moved lines and none added** (444 → **444**): `U10 documents`'s map digest `48e37378…` → `11873cad…`;
`U10 census`'s §2 figure **24 `open` in §2 → 9 §2 rows carry the record's own `*`**; `U10 bar`'s head digest
`48e37378…` → `11873cad…`; and `U10 in place`'s declared `48e37378…` → `11873cad…` with its tampered
`70abbf06…` → `610f13c7…`. Every other line is byte-identical — `U10 tree`, all 43 `U10 row:` lines,
`U10 rows`, `U10 gate` and `U10 owed references` included — and no entry, verdict or citation moves: the map
digest is re-pinned (`MAP_DIGEST`), the four undecided rulings and `mc-ar-mod` stay owed, and the suite stays
**226 passed / 0 failed** (the SC-001 check gained assertions, not a function).

**The census's own split is now read off the map's rows, not written beside them (2026-10-12, user).** The
one figure on the census line that was still a literal — the per-section partition `10 §1 + 42 §2 + 19 §3 +
35 §4` — is now **derived at runtime** from `docs/MODS-MAP.md`'s own rows. `gameplay::MapRows` is the
module's **one** parse of the map's tables (the parse SC-001's check used to carry beside it, moved to where
the tree can read it: the test lost a parser rather than the module gaining one); `Tree::of_record` reads it
**in place** and carries the four digits, refusing `source_absent` by name when the document is not there;
and `Dispositions::render` **takes the split as an argument**, so no literal on that line can disagree with
the rows it is printed beside. The digest pins are unchanged and stay where they were — the row's own declared
source, verified by the gate at admission (`source_digest_mismatch`) — which is what lets a **moved** row be
visible in the printed split instead of hidden behind a digest. **No stdout line moved: 0 added, 0 removed, 0
changed** (444 → **444**, `cmp` byte-identical to the capture taken before the change): the change is where
those digits come from, not what they are. It is proved **live** by moving §3's `shuttfup`/`Stfu` row into §4 —
the census line then prints `10 §1 + 42 §2 + 17 §3 + 37 §4`, the end-to-end check fails by name
(`[10, 42, 17, 37]` against the pinned `[10, 42, 19, 35]`), the new unit check fails by name with the same
two vectors, and the binary exits 1, refusing the moved document by its own `source_digest_mismatch`. The map
was restored **byte-exact** (sha256 = `11873cad…`, the pin) and every gate reran green: **228 passed / 0
failed** (two tests more than the 226 of the correction above — the census's print-is-the-argument check, and an
absent map's refusal by name),
clippy 0 diagnostics, binary exit 0, `find crates` = 0.
**The census's seam declared — and the four rulings put to the user (2026-10-12, user).** `contracts/u10-seams.md`
now carries **C6**, the one seam this slice exposed without declaring one: the `Dispositions` counts,
`Tree::sections()` read in place, `render(sections)` taking the split as an **argument**, and `MapRows` as the
module's one parse of the map's tables — with two contracts of its own. *First*, nothing on the print path may
state the partition: the digits are handed in, so a row moved across sections moves them and the check that pins
them fails by name. *Second*, this reader deliberately **does not verify the pin**, and where the pin *is* enforced
is named rather than left implicit: the row's own declared source, verified by the gate at admission
(`source_digest_mismatch`, on which the binary's exit status depends) and by
`the_declared_digests_are_the_documents_on_the_disk` — measured by appending one byte to the map, which left the
printed digits unchanged and refused the run at exit 1. The file's header had miscounted its own seams (*"three"*
beside the five sections it already carried); it now names the six it declares, the correction dated in the header
itself. The four owed rulings — the recipe row kind's key and version, the refusal's name for a second row on a
descendant, a verb that serves the whole tree, and the per-row family assignment with the `SUBSTANCES` shape — and
`mc-ar-mod` with them were **put to the user as questions the same day, and no ruling was returned**: all five
**stand owed**, unchanged, and the contract's owed list now says so rather than leaving a reader to think they were
never asked.

**What is owed of U6, unchanged and not re-opened**: the document record and its compression, the calibrated
structural judge, the ballot, the permission policy matrix, the API document, a pin over a request line, the
crate × axis census over the tree, the bijection of the served table against the legacy action vocabulary,
`save`/`load` as rows of the table, the served spelling of an owed or unnamed requirement, the symlink
question, the table print's missing claims, and §22's un-re-derived rows.
---

## 24. U5's first slice — one sensor concept, one band table, and the onset the audio asked for (2026-10-12, fourteenth pass)

**Row 5's first half is built.** The slice is **U5** — instrumentation, light and audio — taken on the plan's
own order after U10 (§23), with U6's remainder untouched and U10's four owed rulings (plus `mc-ar-mod`) left
owed. **Created**: `clean-room/src/measure.rs` (554 lines), `clean-room/src/light.rs` (911),
`clean-room/src/audio.rs` (271), `clean-room/tests/u5_measure.rs` (297). **Changed**: `src/refusal.rs` (four
kinds — `not_weighable`, `not_a_count`, `band_inverted`, `unwatched` — the closed vocabulary declared
**65**, `ALL`/`COUNT`/`name()` together *(corrected 2026-10-12, §25: **65 → 77**, the twelve kinds U7's
slice added; the pinned count in `tests/u5_measure.rs` moved with it, by name)*), `src/lib.rs` (three
modules), `src/evidence.rs` (`check_u5`, thirteen
lines), `src/main.rs` (the bar joins the gate), `src/systems.rs` (U5's real status). No dependency was added
(`sha2` remains the only one), **no file under `crates/` was read** (P7), and the three witness crates were
read for **behaviour only** — the gate strings and the published tables — with no line copied from any `.rs`.

**Derived from, by document.** `docs/DESIGN-SPEC.md` §3's slots S-03, S-12, S-13 and FC-2.16 (whose four gate
strings `LEDGER.md` C-25 carries, quoted in `measure.rs`'s header rather than paraphrased) and §5's scale
contract (the voxel is 1 m, so one unit of stock reaches a density's own g/cm³ through `UNITS_TO_CM3` = 10⁶);
`docs/LEDGER.md` C-93.5/C-94 (the onset layer exists because the world's precipitation is mm/year and nothing
said when rain *starts* — *"an audio layer that wants is it raining right now had nothing to read"* — and
C-94's six-constant table is this module's own table); `docs/light-solver.md` §1–§6 (the closed form, two
media two geometries, the twilight's two curves, the Moon as a reflectance with its π, the droplet optics,
the response's two calibrations, and §6's owed format); `docs/missing_atmosphere_deepseek.md` §1.5/§4.2/§4.5
(Ångström's exponents, the anchored twilight curve, the night sky's own 0.001 lx); and the published tables
where they are used — Bodhaine et al. (1999)'s standard atmosphere (`1.16e-5 m⁻¹` at 550 nm), Kasten & Young
(1989)'s air mass, Wyman/Sloan/Shirley (2013)'s single-lobe fit, Henyey–Greenstein at the record's own
`g = 0.85`, Koschmieder's `3.912/V`, the solar constant `1 361 W/m²`, Planck at `5 778 K`, and the Moon's
own `R`/`d`.

**The interface, in the plan's words**: `Sensor`/`Band`/`GaugeReading`/`GaugeWall` (**a reading is a band,
never a score** — three words, no number between them), `Solver`/`Spectra`, and the audio seam. **A quantity
that cannot be weighed is refused by name**: `not_weighable` (nothing crossed the boundary, so there is no
quantity there — answering `0 g` would be a measurement the gate did not make), and the four neighbours each
in their own name rather than one symptom for all — `unadmitted_material` (no density to weigh it through),
`not_a_count` (a mass at a band: S-12's band is per *item*), `unwatched` (a reading no gauge answers for),
`band_inverted` (a band with no inside never held a reading).

**Measured by the binary's own thirteen lines** (`cargo run --offline`, exit 0):

* **Instruments**: 3 virtual gauges in one controller; band `32..=96` reads `64→within`, `31→below`,
  `97→above`; the gate's answers exact — 3 units of quartz `7 950 000/1 g`, 64 of basalt `192 000 000/1 g`;
  all five refusals `true` by name.
* **The table**: 12 bands over 400–700 nm, **calibrated** to the published `1 361 W/m²` (the scale
  divides the Planck shape's own total, so the sum is a calibration and not a measurement); the target is
  pinned as a **literal** in the checks, proved below by moving the constant. A flat spectrum renders
  `(0.08333, 0.08333, 0.08333)` — neutral to `4.55e-7`, the quadrature's own seam error — and the Sun
  renders `(0.080, 0.088, 0.086)`. **The daylight sky at 60° reads `(0.039, 0.079, 0.143)`: blue over red
  3.6×, from `λ^-4.09` alone.**
* **Fog**: `0.021 g/m³` is `0.895 km` of visibility; the beam falls `1185.90 → 969.13 W/m²` while the sky
  rises `21.23 → 120.56` — *the light is not lost, it is moved into the line of sight*, measured.
* **Twilight**: `400.0 lx` at 0°, `73.07` at −2° (the reference prints ~80), **`13.35` at −4°** (prints ~15),
  `2.44` at −6° (prints ~2.5), `0.0149` at −12° (prints ~0.01), `0.0010` at −18°; the curve meets the night
  sky's own floor at **−15.18°**, before astronomical twilight's −18°.
* **Beam and night**: `94.75 W/m²` at +0.1° and **exactly `0.0` at −0.1°**; the sky at −18° reads `5.712e-6`
  against the night sky's own `5.593e-6` — 2.14% of scattered light above the floor, so a night is dim and
  never an absence.
* **The Moon**: `2.8519e-3 W/m²` full (the record's own measurement on its table is `3.0484e-3`), `1.95 lx`
  at 683 lm/W and **`0.29 lx` at the Moon's own ≈100 lm/W** — the published full-moon figure; doubling the
  Sun doubles it exactly, a tenth-lit crescent is exactly a tenth, an occluded Moon is exactly zero; `Ω` =
  `6.4177e-5 sr` = `π(R/d)²`, and the legacy's own `(R/d)² = 2.0428e-5` is **π too dim**, named rather than
  inherited; sun ÷ moon `4.772e5` against the record's own six orders (`4.5e5`).
* **Audio**: the onset read at `(1234, −55)` t7000 is `0.0290` (gain `0.000`, not raining); one value per
  2 000 ticks and per 900 m; **this field's own duty `0.0482` against the record's own measured `0.0997`**,
  printed beside it rather than fitted to it (a factor of about two drier, which is this build's noise).

**Three defects the measurements found, each fixed at the cause rather than bounded away.**

1. **The field's own shape.** A ridged field rained **0.9858** of its samples — a world where it is always
   raining, which this world's mm/year is not. The field is now *rain as a positive anomaly*: where the
   tick-noise field is not positive there is no rain, and half the field is dry by construction, with the
   record's own `ONSET_DRYNESS` sharpening what is left. The duty is then a *measurement* (0.0482), not a
   bound raised until the assertion passed.
2. **The sky's scattering share.** Read off the per-metre coefficients, a droplet load with **no layer**
   still changed the sky, so *"no droplet layer declared is no path"* — the record's own fix — was true for
   the beam and false for the sky. The share is now read off the **column's own optical depths**, and the
   test that found it (`the_beams_droplets_are_their_own_layer_and_not_the_whole_column`, which asserts the
   no-layer case byte-for-byte) holds both halves.
3. **A bound below the physics' own slope.** The twilight knee was asserted to 0.1% of the floor, and the
   curve's own gradient is `0.85/°` — a hundredth of a degree is already 0.85%. The bound is now the slope's
   own, and the comment says so.

**What U5 owes, named and not silently missing**: **ozone** absorption; an **azimuth** (this solver is one
plane through the Sun); the per-cell **frame format** (`docs/light-solver.md` §6's remaining step, and a
renderer change rather than a solver one); the audio's own **voices** and the **music/ambient beds** the
mission's feeling-wheel work will drive (C-94 names the seam's first half — *is it raining right now* — and
this is it); and the record's own **`WET_TURBIDITY_GAIN`** link between a wet column and a hazier one, which
is a caller's mapping rather than this solver's arithmetic.

**And the cascade.** `cargo test --offline` — **260 passed / 0 failed**, of which **32 are this pass's own**
(26 library: `measure` 9, `light` 12, `audio` 5; 6 integration in `tests/u5_measure.rs`). The 228 that stood
before them is two more than §23 recorded (226); this pass wrote neither and does not guess at them — the
arithmetic is named so a reader can check it. `cargo clippy --offline --all-targets -- -D warnings` exits 0.
`cargo run --offline` exits 0 at **457 lines of stdout** (444 → **457**: **thirteen lines added and none
removed**); the **two** pre-existing lines whose *text* moved are the registry's own `[U5]` row and its
`interface:` line (which gains `Crossing; Mass (and nothing else);` and `(onset, gain)`), and every other
line — the seven `U10` lines included, with their recorded digests (`11873cad…`, `6ddffde0…`, `426b58d7…`,
`69 row(s) declared by 106 entr(ies)`) — is unchanged, checked against the figures §23 recorded.

**And the four owed rulings** (§16.3's three, plus `mc-ar-mod`): they **were put to the user as questions and
no ruling was returned**, so all of them stand **owed, unchanged**, and this pass decides none. Where a
choice could not be avoided, the **smallest reading** was taken and named: the onset field's own shape is
this build's (measured and printed beside the record's figure), the droplet layer's depth is *declared* (40 m
for `docs/light-solver.md`'s "tens of metres"), and the field's own scale (2.0) is declared for a **unit
mismatch** — the record's thresholds are stated in the field's own units and an fBm's spread is not those
units — rather than fitted to the record's duty.
**The audit that followed, and its five findings (2026-10-12, user).** A read-only audit re-derived this
pass's report from the code and files and made five findings — the delta's accounting, the unpinned
constant (P2), the calibration sentence, the duty band over names (P4), and the public surface. All five
are closed, each by making the guard bite by name or by stating what was not true, and **nothing else
moved**: no row's verdict, citation or entry set; the four U10 rulings and `mc-ar-mod` stand owed and
untouched.

1. **The stdout delta's changed lines are two, not one.** The audit's byte-level diff against a surviving
   pre-change capture (444 lines, taken before this pass's first run) reads **13 added, 2 changed, 0
   removed**: the registry's `[U5]` row *and* its `interface:` line, which gains `Crossing; Mass (and
   nothing else);` and `(onset, gain)`. The sentences above are corrected in place.
2. **The twelve-band sum is a calibration, and its target was unpinned.** The published `1 361 W/m²` is
   now pinned as a **literal** in three places — `light.rs`'s own check, `tests/u5_measure.rs`, and the
   binary's `light_ok` — so a moved `SOLAR_CONSTANT_W_M2` fails by name instead of scaling with the
   assertion. Proved: the constant at `1 300` leaves `cargo test --offline --no-fail-fast` (exit 101) red
   at `light::tests::the_twelve_bands_sum_to_the_published_solar_constant` **and** at
   `the_light_solver_reads_one_table_and_its_numbers_are_the_records_own_claims`, and the binary exits 1
   printing `1300.000000 W/m² … WRONG (the calibration's published target moved)`; restored byte-exact
   (sha256 OK).
3. **The audio duty was a factor-of-four band over names.** C-94's seven constants are now read as
   **values** against the ledger's own table, and this field's duty is pinned to its measured `0.0482` on
   both the module's side and the export proof's — the record's `0.0997` stays quoted beside it. Proved:
   `WET_ONSET` at `0.26` leaves the suite (exit 101) red at
   `audio::tests::the_fields_duty_is_measured_and_not_assumed`,
   `the_audio_seam_answers_is_it_raining_right_now` and
   `the_derivation_documents_are_where_the_modules_say_they_are` (the row no longer reads `0.18`), and the
   binary exits 1 printing `0.0306 … WRONG (the field's own pinned duty moved)`; perturbing **the record's
   own row** (`0.18` → `0.19` in C-94) fails the same document check by name; restored byte-exact.
4. **The public surface reachable only from inside its own module is cut.**
   `Droplets::scattering_per_m`, `Solver::view_tau` and `GaugeWall::gauges()` are **deleted** (no caller
   anywhere; `Gauge` is now the wall's private element, with no accessors of its own), and `Mass::to_f64`,
   `Spectra::with_air` + `Air::hazy`, `GaugeWall::band` and `Channel`'s three accessors are **no longer
   public** (test-only or internal). Three named instances are **refuted** and stay:
   `Spectra::with_sun_factor` is called by the binary's own doubling read, and `Droplets::layer` and
   `Weather::is_silent`/`is_wet` are called by `tests/u5_measure.rs`, the export proof — they are not
   reachable only from inside their module.
5. **The record's figures are the code's figures.** The stdout is byte-identical to the pre-audit capture
   (444 → 457: 13 added, 2 changed, 0 removed); the suite is **260 passed / 0 failed**; clippy exits 0;
   the binary exits 0 at 457 lines, 13 of them U5's.

---

## 25. U7 — the decision layer: the stage is in the record's head, and a bare `A` is not a measurement (2026-10-12, fifteenth pass)

**Row 6 is built.** The slice is **U7** — axis · laya · von, the join Lane C names — taken on the plan's own
order after U5's row 5, with U6's remainder untouched and U10's four owed rulings (plus `mc-ar-mod`) left owed.
**Created**: `clean-room/src/axis.rs` (**811** lines), `src/laya.rs` (**958**), `src/von.rs` (**1655**),
`clean-room/tests/u7_decision.rs` (**445**) — all four re-measured on the final bytes (§26). **Changed**: `src/refusal.rs` (the closed vocabulary
**65 → 77**, `ALL`/`COUNT`/`name()` together), `src/lib.rs` (three modules), `src/evidence.rs` (`check_u7`),
`src/main.rs` (the bar joins the gate), `src/systems.rs` (U7's real status, its interface and its slice row
corrected `7` → `6`), `tests/u5_measure.rs` (its pinned count moved to **77**, with the twelve kinds named).
No dependency was added (`sha2` remains the only one), and **no file under `crates/` was read as code** (P7):
the four witness crates were read for behaviour only — the published vocabulary and the recorded numbers.

**Derived from, by document.** `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U7) and §5 row 6;
`docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` Lane C (the missing join, and a decision as *"a
subject, an evidence, a rollback"*); `docs/LEDGER.md` R-59.5 with §59.5 item 12 (the bar is the **majority
class**, the set may not be edited to manufacture a pass, the measurement is reported either way), C-102
§102.2 (why the majority class is measured), R-59.7 (the total pin and the anti-fabrication guard), C-73
§73.1 and C-97.2's made rule (the axis prohibitions); `docs/HYPERGRILLING.md` H-14.1 (*the model instrument
is calibrated and quotable only when it beats the majority class*); `patches/PATCH-023-the-ballot-and-the-von-classifier.md`
§1–§3 (the ballot's fields, the floor declared before the answer, and the three rules — the origin is the
proof's to carry, a shape mismatch is a non-reading, a missing answer is a non-reading); and
`patches/PATCH-039-the-corpus-beyond-this-tree.md` (the five stages, the sixteen shared ids, and the sentence
this pass's correction rests on).

**The design correction, which is the finding.** The first draft read the record's **body** — its claim —
and deliberately kept the head out of reach so the reading could not restate the label. The measurement
refused it: over the sixteen labelled records the body-only classifier scores **0.0000** leave-one-out
against a majority class of **0.3125**, and the reason is a property of the corpus rather than of the
estimator — every one of the sixteen claims declares the same four sections and quotes the record's own
vocabulary, varying only in length (19–53 lines). **The matrix is now pinned by a test**, so a later edition
cannot quietly forget what the first draft was reading. The stage is in the record's own **life facts**, and
`patches/PATCH-039` says exactly that of this corpus: the same id at five stages across five trees,
*"answerable per record, from ids and heads alone"*. The question moved to the head's declared fields — with
the label's own key read by **no** feature, so the reading is a reading rather than a restatement — and the
claim-only figure stays on the record as the losing half the rules require to be printed.

**Measured by the binary's own seven lines** (`cargo run --offline`, exit 0):

* **The axes**: 7 ladders — **3 declared** and **4 owed by name** (`certification`, `org_rank`,
  `gate_status`, `evidence_level`); the coverage table describes every axis exactly once; a forged row
  refuses `fabricated_axis` and an axis with no row refuses `uncovered_axis`.
* **The prohibitions**: a cross-ladder comparison refuses `axis_interchange_refused` while two members of one
  ladder compare freely; **14** reductions refused (7 axes × 2); a bare `A` is `AMBIGUOUS` across **3**
  declared ladders, a bare `SSS` is `UNQUALIFIED`, and a token no declared ladder claims is not this rule's
  business.
* **The vocabulary**: **8** declared words read over **39** record(s) **in place** — `strengthen`×4 (+1
  outside the calibration set), `replace`×1, `design-approved`×8, `applied-to-internal-base`×5,
  `user-confirmed`×2, `user-selected`×1, `user-directed`×23 (+23 outside), `failed-eval`×4 — every word's
  claim held, and **24** of the readings are records the calibration was **not** taken over.
* **The calibration**: 16 readings, `proposed 5 / designed 4 / verified-internal 4 / applied-internal 1 /
  approved 2` — **0.7500** in sample, **0.5000** leave-one-out, **0.4375** leave-one-out with absence,
  **0.0000** leave-one-out from the claim only, majority class **0.3125**, margin **+0.1875**; the bar is the
  majority class read strictly = true; the stored labels still describe the records = true; the set's own
  digest `d803c643`.
* **The ballot**: `u7-life-of-PATCH-015` over `patches/PATCH-015-hypergrilling.md` → `belongs`, origin `A`
  (`Oracle`); the reading is `approved`; **3** declared feature(s) held; 5 option(s) scored; the evidence is
  `21aa66cd` (read in place).
* **The reading's mechanism, named**: the facts that carry a stage are printed with their lifts —
  `approval_is_user_confirmed→approved +0.58`, `long_body→approved +0.58`,
  `approval_is_applied_to_internal_base→verified-internal +0.50`, `operation_is_strengthen→applied-internal
  +0.44`, `declares_no_dependency→verified-internal +0.39`, and so on — with a column that **no** reading
  holds excluded, because its lift is an artefact of smoothing over an empty column and printing it would
  name a mechanism that does not exist.
* **Fail-closed**: an uncalibrated instrument writes nothing [`not_established`] = true; a missing instrument
  is refused [`oracle_unreachable`] = true; a non-reading is never a pass [`no_reading`] = true; a
  `not_belongs` reading needs a human's reason in the same act [`reason_absent`] = true, and with one it is
  filed carrying the reason = true.

**Each guard proved to bite, live, and restored byte-exact** (the sha256 of the touched file checked before
and after; four perturbations, four named refusals, four exit-1 runs, four restorations):

| Perturbation | What refused, by name | Restored to |
|---|---|---|
| `patches/PATCH-013` `status: designed` → `approved` | `claim_unverified` — *stale rather than wrong, so the record is re-read and the set is **not** edited to fit it*; the binary's own line reads *the stored labels still describe the records = false* | `1d403eea6cad…1846be` |
| `patches/PATCH-023` `status: applied` → `proposed` | `claim_unverified` naming the word: *carries the declared word `user-directed` and reads `proposed` … the claim is a property of the format*; `every word's claim held = false` | `7dc2b8fb2d41…b76b76d` |
| `patches/PATCH-015` `approval: user-confirmed` → `design-approved` | the **reading** moves to `proposed`, so the ballot's floor is missed and the answer is refused `reason_absent` | `21aa66cdd1c2…83ee2c` |
| `patches/PATCH-016` moved out of the tree | `source_absent`, naming the path: *a calibration is measured over the records this tree actually carries* | present again, `bcb935b8e113…0d8c60` |

**The order of the four rules is declared rather than inherited.** `laya::answer` reads the bytes, asks the
instrument, and only **then** reads its calibration — because asking first is what keeps *unreachable*
distinct from *uncalibrated*: an instrument that cannot run reports no readings at all, so checking its
figures first would answer every missing oracle with the wrong rule's refusal. The reasoning is in the
function's own doc comment, and the perturbation table above is the proof that neither rule stands in for
the other.

**And the cascade.** `cargo test --offline` — **323 passed / 0 failed** *(re-measured 2026-10-12, §26)*, of which
**63 are this slice's own** (46 in the three modules — `axis::tests` 13, `laya::tests` 13, `von::tests` 20 —
and **17** in `tests/u7_decision.rs`, the extra one the audit's closure added). One pre-existing test **moved
rather than being edited to pass**:
`tests/u5_measure.rs::every_refusal_is_named_and_the_vocabulary_round_trips` pinned the closed vocabulary at
**65**, and this slice made it **77**; the test now reads **77** with the twelve kinds listed by name, and §24
above is corrected in place with the date. `cargo clippy --offline --all-targets -- -D warnings` exits 0 (one
`needless_range_loop` was fixed at its cause — the column loop became an iterator over the slice it was
indexing — rather than allowed). `cargo run --offline` exits 0 at **464 lines of stdout** — **measured
directly**, and unmoved by the correction pass below (none added, none removed) — and every non-U7 line's
published digest (`11873cad…`, `610f13c7…`, `69 row(s) declared by 106 entr(ies)`, the three `U10` lines'
own figures) is still on the output, checked after the run. **The `459` this section first reported is
*derived*, not captured** *(corrected 2026-10-12, §26)*: it is §24's 457 plus the two U7 lines **this pass's
own first draft** printed — **not** "the previous pass", which printed no `U7` line — and **no capture of a
459-line state exists anywhere in the tree**. The only line count a reader can re-measure is the **464**.

**One disagreement found while wiring the registry, and named rather than smoothed.** `src/systems.rs`'s
`slice` field is the registry's **table position** — its own test asserts `slice == i + 1` for every row,
which is what makes the table closed and ordered — while the plan's §2 entries use `slice:` to name the §5
**row**, and row 5 carries U5 and U6 together, so the two readings differ for U6/U7/U8 by §16.3's own
correction. This pass had briefly set U7's field to `6` and the registry's test refused it by name
(`systems::tests::the_registry_is_closed_and_ordered`); the field is back to `7` with the difference
explained in a comment **at the field** rather than left for a reader to find.

**What this pass deliberately did not do.** It did not touch the known-answer set, move a label, or choose a
feature because it made a figure larger: the head's features are the format's own declared words, and the
word→stage claims the features rest on are measured over all 39 records with the count *outside* the
calibration set printed beside each (24 readings). It did not decide the four U10 rulings or `mc-ar-mod`, and
it did not re-open U6's remainder; it did not read any `crates/` source as code, and it added no dependency.

**What U7 still owes, named and not silently missing**: the **score instrument's rungs** (a score question
parses and refuses a floor outside its rungs, but no instrument here reads rungs, so it is a **non-reading**
rather than a verdict); the **`noul` and `score` calibration sets** (only the stage question has labelled
examples in this tree, so the three-part question has **no set to be calibrated against** and is **refused by
name** — `not_established` — rather than quoted *(corrected 2026-10-12, §26)*: an answer would carry the stage
question's figures, a measurement never made for it); a **model-backed instrument** behind `StructuralJudge` (the legacy ONNX session's
slot — `Unreachable { what: … }` is the shipped witness that a missing instrument is refused); the four
ladders' token lists, which are U12's vocabulary to read; and `docs/LEDGER.md`'s **C-187**, filed with this
pass's own record.

---

## 26. The U7 audit's five defects, closed in place — the uncalibrated question refused, the unread field cut, three figures corrected (2026-10-12, sixteenth pass)

A read-only audit of §25 (`axis.rs`, `laya.rs`, `von.rs`, `tests/u7_decision.rs`, spec 009, §25 and **C-187**)
re-derived every figure from the code, the corpus and the files. It returned **five** defects, each proved from
the code or the files, and this pass closes each by the **smaller** reading — no capability is invented, no
set is manufactured, and nothing else moves.

1. **A `noul` ballot was answered as a quotable `Origin::Oracle` verdict resting on the *stage* calibration.**
   The audit proved it through the public surface — `laya::answer` on a `noul` ballot returned `belongs`,
   `origin A`, carrying the stage question's 16 readings and 0.5000 leave-one-out. The record already said a
   `noul` answer is not quotable, so the **code** was wrong, not the sentence. **Closed at the instrument**:
   `Classifier::judge` now **refuses a `noul` question by name** (`RefusalKind::NotEstablished`) — this build
   ships no known-answer set for it, so there is nothing for an answer to rest on — and its calibration set
   stays **owed** rather than invented. `tests/u7_decision.rs` proves the refusal **end-to-end through
   `laya::answer`**, and `von::tests`' three-part test is turned into the refusal it now is. *Proved to bite by
   name*: reinstating the answering arm fails `u7_decision.rs:383` and `von.rs:1647`, and the file was restored
   byte-exact (`src/von.rs` `dcc27155fc9f9d4db46802b0ea0c92fff8af2b374135db0087e4a51a27683156`).
2. **`Ballot::max_age_ticks` was declared and read by nothing.** **Cut, not given a reader**: the `Ballot` no
   longer stores the window and its accessor is gone; `Ballot::declare` still **refuses a `max-age` of zero**
   (*not a window*, `patches/PATCH-023`) — the value is checked at declaration and **not carried**, because this
   build cannot enforce staleness and a field nothing reads is not one a ballot should claim to hold. The
   enforcement is **owed by name**, and `spec.md`'s FR-004 says so.
3. **`src/von.rs` was recorded as 1659 lines** in §25, `specs/009-u7-decision-layer/plan.md` and
   `quickstart.md`; the file was **1662**. **Corrected to the measured value**, and — because this pass edits
   the three modules — every listed count is **re-measured on the final bytes**: `axis.rs` **811**, `laya.rs`
   **958**, `von.rs` **1655**, `tests/u7_decision.rs` **445**.
4. **The `459` stdout baseline was mis-attributed.** No capture of a 459-line state exists anywhere in the
   tree, and §24 — the pass it was attributed to — printed **no `U7` line**. The record now says plainly that
   **464 is measured and 459 is *derived* from an unrecoverable baseline with no capture** (§25's correction).
5. **`src/lib.rs` still described `von` as reading the record's body.** **Corrected** to the head (and to the
   `noul` refusal), so the module register no longer disagrees with the code it describes.

**Two things the audit found that are recorded rather than repaired.** `clean-room/` is **untracked in version
control**, so **no provenance claim about it can be made from VCS** — `git` holds nothing for this slice, and
every "restored byte-exact" claim here rests on the **sha256 in the record**, not on a clean working tree. And
the audit could not locate the **`F5–F7` item** it was asked to check **by that name**, in `docs/` or in
`clean-room/`; it is left alone and noted here rather than guessed at. (The nearest reading is
`docs/DESIGN-SPEC.md`'s foundation layers F5–F7, which this slice does not touch.)

**What did not move.** No other row's verdict, citation or entry set; the four U10 rulings and `mc-ar-mod`
stand **owed** and undecided; U6's remainder is untouched; **no second parser and no duplicate owner** appeared
(the record's fence is still read in one place); and the audit's other observations — the dead
`Ballot::instruction`/`presenter` and `Question::passes` accessors and the `no_reading` parenthetical in
`tests/u5_measure.rs` — were **not** among the five and are left exactly as they were.

**Measured on the final bytes.** `cargo test --offline` — **323 passed / 0 failed** (263 library + 60 across
the nine integration suites, of which U7's own are **63**: 46 in the three modules and **17** in
`tests/u7_decision.rs`); `cargo clippy --offline --all-targets -- -D warnings` exits 0; `cargo run --offline`
exits 0 at **464 lines of stdout**, **byte-identical to the pre-correction capture** — **no line added, none
removed** — with U7's seven lines still at 457–463. The one guard this pass changed is proved to bite by name,
and its perturbation was restored byte-exact. The corrected files, by digest: `axis.rs`
`deb13ac3aed11f3498658510886d03e82d7977285590ab41347a20c0a51cee66`, `laya.rs`
`440c45ea202a3cbf4b6ecd1e87cb517e00253de604efaf4a230b74d2acf0c4bb`, `von.rs`
`dcc27155fc9f9d4db46802b0ea0c92fff8af2b374135db0087e4a51a27683156`, `lib.rs`
`799a9d89d554890603b3a4c1fe6e2d78795ca2bca898d28de619f56942492712`, `systems.rs`
`b56174c48d9711250b299d1639c0b10c3aecb6f1ba25a97d0f29cc92d42e3329`, `tests/u7_decision.rs`
`8dbaf1c489a87b3bc67eddeb9367dbe994ccfd430d74531533f60177e555ff41`.

---

## 27. The sixteenth pass — U8, the shell: a visibly drawn frame, and one pointer decision

**What ran.** U8's spec set was created under `clean-room/specs/010-u8-shell/` (spec, plan, tasks,
quickstart, research, data-model, `contracts/u8-seams.md`, and the requirements checklist) from §2's U8
entry, §5's row 7, `docs/SHELL-PORT.md` §1/§4/§6 and `docs/UI-SPEC.md` §1–§5/§8/§9, and the slice was
implemented in one new module, `clean-room/src/shell.rs` (**1406** lines), with the three refusals it needs
added to the closed vocabulary and the binary's new `shell` mode.

**The bar, and what reached it.** §5 row 7's check is *a **visibly** drawn frame; pointer ownership read
once* — and §2's interface sentence is the same bar as obligations: *a `Mount`-shaped pane tree driven by a
declared `Recipe` (not a hard-coded app registry), the two layers' toggle, one pointer-ownership decision
read once and applied to both grab and visibility, and engine constants sourced from an admitted table
rather than literals.* Each is a measurement, not a claim:

* **A visibly drawn frame.** `shell::frame` draws into a character `Canvas` and prints it. The declared
  extent is **100 × 30** (`shell::declared_area`), the frame's digest is
  `79f3885c50beaf5c715aab995be36892052f2b0158b179240bcb67722e4f0d58`, and a whole interactive session is
  kept at `docs/baselines/u8-shell-session.txt`. The binary's new **`shell` mode** draws the frame and reads
  one command per line (`split`/`close`/`focus`/`layer`/`grab`/`quit`), redrawing after each.
* **The mount, and no second registry.** `Mount::of(&Recipe)` mounts the recipe's **six** windows and adds
  none it was not handed; `the_recipe_is_the_only_window_list` holds it.
* **The two layers' toggle.** `Layer::{Shell, Hud}` over **one** window set; the HUD draws the two windows
  declared `pinnable` — the reference's own defect fixed (*"shaper UI hidden when esc menu hidden or f1
  toggled"*, `docs/UI-SPEC.md` §1).
* **One pointer-ownership decision, read once.** `Mount::read_pointer` computes the value once and both
  consumers take it **by value** — the drawing and the grab — so `visible` cannot disagree with `owner`; the
  signature, not a comment, is the mechanism.
* **Engine constants from an admitted table.** **16** rows across the four buckets (md1-rule **8**,
  md1-example **2**, platform-translation **1**, project-extension **5**); an unadmitted name refuses
  `constant_unadmitted`; the frame's own geometry reads the table and the bar prints its derivation
  (`bar 6 cells (48 dp ÷ 8 px) floor 5`), so no literal hides in the drawing.
* **The panes show real state, not a mock.** The six windows read the systems already built: the registry
  (`13` systems), the reference world (`seed=1592639215`, its truth and residency digests), its materials
  (`6` admitted), the stage roster (`15` stages), a derived chunk's geometry (`1031 cells → 144 quads`,
  merge factor `7.16`) and the light table (`1361.000 W/m²`).

**What moved, and by how much.** `RefusalKind::COUNT` moved **77 → 80** with this slice's three kinds
(`constant_unadmitted`, `pane_unknown`, `split_floor_breached`), cascaded into `tests/u5_measure.rs` by
name. The library tests moved **263 → 274** (eleven `shell` tests) and the total **323 → 334**. Stdout moved
**464 → 467**: **three** lines added (U8's frame, digest and constants lines), **none removed**, and
**three** lines changed **in place** — the `[U8]` registry line (`absent` → `built`) and U8's `interface:`
field (this pass), and U13's status sentence (the fix pass below: *"which no slice owns"* → *"which its
own acceptance check closes"*). Both captures are kept beside this record —
`docs/baselines/u8-pre-stdout.txt`
(`sha256 8dbce319a305c90a14420bc2bcd7ffc5b7510ed82c5073d4dfd498dfa24f0efe`) and
`docs/baselines/u8-post-stdout.txt`
(`sha256 b2363fe7acb7311f54570eb674824f8c40d1870b004b13772891e0e0db9362bc`) — so the next audit can diff them
line by line rather than trusting a count.

**Measured on the final bytes.** `cargo test --offline` — **334 passed / 0 failed** (274 library + 60 across
the nine integration suites); `cargo clippy --offline --all-targets -- -D warnings` exits 0; `cargo run
--offline` exits 0 at **467 lines of stdout**. Digests: `shell.rs`
`a57b53307de8577b60f30802cf51a6b6b9b8dafee3038e657ab9801a981f32fb`, `refusal.rs`
`f70369e9f9c2b8c6b9d1136b9e89e58560a5e10340461629f681a04b80f6cd4b`, `evidence.rs`
`31ef69ca5744c5f2ce8a907f30f61d4e8efb4c7818700482e9ed252ccbdc40bb`, `main.rs`
`e598e4bf9dd682ca712713ad2edb39ae5861aa7f51ccc00082781885e990e9cd`, `systems.rs`
`b1cdf3f5c202f948b929cd786ac8a753bb476ae2b60b2084dc2058b0895854ab`, `tests/u5_measure.rs`
`4c2205b7cba59442bc33d30b8c173e9733e7fd77d83211efd3fcaca05979b401`, `src/lib.rs`
`dc92774eeab8828bcdeb4c2ab5a712d5a94ebe05e897406ecfc040fff53450f9`.

**The `gholtzap/lintapple` gate is U13's, and this pass says so rather than deferring it silently.**
`clean-room/specs/001-presentation/plan.md` §3 carries the row **`P8 … WIP | NEEDS WORK`**, §5 row 13
defines **U13** as *"the shell completed, and its gholtzap/lintapple gate"*, and `systems.rs`'s own U13
entry names the gate as a wait. U8 is the shell; U13 is *the shell completed* plus the gate, and it is last
by §2's own reasoning (*"completed first, it would be judged against nothing"*). Taking the gate here would
duplicate U13's bar, so this slice does not — and what it contributes to that gate is the half the gate
checks, **every constant admitted**, which is the constants table. The statement is repeated in
`clean-room/specs/010-u8-shell/spec.md` (its own section), so the deferral is on the record in both places.

**What did not move.** No other row's verdict, citation or entry set; **U1's remainder**, **U6's
remainder**, **U11**, **U12** and **U13** stay exactly as they were; the **four U10 rulings**, **`mc-ar-mod`**,
the **`noul`/`score` calibration sets** and the **score instrument's rungs** stand **owed** and untouched;
no second parser, renderer or pane tree appeared — the fence is still read in one place, and `shell.rs` is
the clean room's only pane tree; and the audit's finding that `clean-room/` is untracked in version control
is unchanged, so no VCS provenance is claimed for this slice either.

**What remains owed after this slice.** U8's own remainder — **undock**, the **splitter-drag resize**, the
**console's five graded levels**, the **toast's final form** — and the **P8** gate (U13's). The plan's order
then continues at §5 row 8 (the Lane A/B/C measurement on the clean room itself).

**Closed by the fix pass (2026-10-12, same day).** The U8 audit's findings were closed with the smallest
reading each: the shell's refusals now read as words (`grab`/`split` say “not `maybe`” and “and none was
given” instead of Rust's `Some(..)`/`None`), a `focus` with no argument refuses `argument_malformed`
rather than `pane_unknown`, the geometry pane calls its 1031 **cells** — U4's own word, whose line
reserves *columns* for the 1024 the field plane produced — with that relabel carried into this section,
`spec.md`, the checklist and `tasks.md`; `Canvas::rows`/`Canvas::cols` and a dead `Bucket::name` were
cut, so the four bucket labels have one owner (`evidence.rs`'s own line); `systems.rs`'s U13 entry no
longer says the P8 gate is one *“which no slice owns”* — it says *“which its own acceptance check
closes”*; and `.specify/feature.json` names `specs/010-u8-shell`. The split-floor looseness was fixed on
the spec's side rather than by adding a second behaviour: `spec.md`'s edge case now says the refusal is
`Mount::layout`'s — at the frame, where the area is known — and that `split` itself accepts the
operation, because the mount does not know the frame's area (`u8-seams.md` C6 and `research.md` §6
already had it that way). Measured: stdout stayed **467 lines**, with **two lines moved in place**
against the capture it supersedes — U13's status sentence and the frame-digest line, whose digest moved
`59509ab2…` → **`79f3885c…`** — and the post-side captures were refreshed to the measured bytes
(`u8-post-stdout.txt` `b2363fe7…`, `u8-post-stderr.txt` `475fd170…`, `u8-shell-session.txt`
`fe7be0a6…`), with the pre capture untouched; of the file digests listed above, only `shell.rs`,
`main.rs` and `systems.rs` moved, and are re-measured there. The four U10 rulings, `mc-ar-mod`, the
`noul`/`score` calibration sets and the score instrument's rungs stay owed and untouched, and no entry,
row verdict or citation was moved. **Observed, not attributed:** `clean-room/src/evidence.rs` (all 2935
lines) and `src/gameplay.rs` carry CRLF terminators while the other 53 `.rs` files are LF-only; with
`clean-room/` untracked in version control, which pass introduced either cannot be established here, and
`core.autocrlf=true` will normalise both on add.

---

## 28. The seventeenth pass — U9's document form: a save consumed in place by reference, and the table's seventh and eighth rows

**What ran.** §16.2's ruling and §5 row 9 left exactly two things owed by U9, and both are the same
capability seen from two sides: *the **document form** of a save — a save consumed in place by reference
(`path + digest`), where U6's content gate becomes a **hard** prerequisite* — and *`save`/`load` as rows
of U6's one tool table*. This pass lands both, in the order they were named, with no waiting on anything:
the gate they were ruled to wait on (§12, §13, §15, §18–§22) is built.

**The one new module, and the shape of the claim.** `clean-room/src/save/document.rs` (**129** lines) holds
[`Document`] — a path, the digest its bytes must hash to, and a length; no bytes and no world, because a
document is not a second save — its **one** writer (`Document::write`, U9's own encoder's bytes placed at
the declared reference under the gate's own root) and `Save::read_document`, the **one** reader of this
form, which reads through **U6's** [`Intake::read`]. There is **no `fs::read` in the file**: *consumed in
place* and *the content gate is a hard prerequisite* are therefore the shape of the code rather than a
policy a caller could forget — and the write half has its own name, `document_unwritable`, because a
caller told "written" about bytes that are not there would be reading a save that does not exist.

**The two rows.** `bridge::TOOLS` is closed at **eight** verbs: `save` (`path:path`, owner
`save.write_document`) and `load` (`path:path`, `digest:digest`, owner `save.load_document`), both by
**reference**, both declaring that they change no record, and both answering facts that are **derived**
from the bytes and the world at verification time (`receipt::document_facts`) rather than carried — so a
call and a re-check of one document cannot drift apart, and a writer cannot edit an answer field into a
document that was never written.

### The measured numbers (from `clean-room/`)

| Claim | Measurement (printed by the binary) |
|---|---|
| SC-001 the document form reads back as the same world | `save` wrote **465** byte(s) at `the-world.save` (`98fc98f1722f942c17f3c72f29dcfa96d35b02e12e00b74e6c6c4e7f110e62dc`); `load` read them back by reference as `truth=32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539`, **the live world's own** (`equal = true`); the write and the read are **one value** = `true` |
| One reader, not two | the library's own door (`Save::read_document`) reads the same document through the same gate = `true`; the module has no `fs::read` |
| The receipts are re-established, not believed | both rows' receipts verify with their carried facts re-derived = `true`; their claims are *none* — the row declares no `claims_hold`, and the answer's facts are re-derived instead |
| SC-004 each fault has its own name, and is recorded | an absent document [`source_absent`], bytes altered inside the save's own trailer [`save_tampered`], and a document that is not the one the reference names [`source_digest_mismatch`] — *each by its own name* = `true`, *each recorded* = `true` (the record moved by exactly **3**), and the record still verifies = `true` |
| The same two rows over the gate's own world (U6's section) | `save` wrote **673** bytes at `u6-world.save` (`611aadcbf6b741976b98b7968bb1eb87fdbd12807f45fa2f50b1960ec80f5c4e`); `load` read them back as `seed=1592639215 period=1s tick=0`, `truth=146ff7bf95369b177f3df90ae837d91c95ff4dbf152d266aa2d8f1e46c381733` — the gate's own; the write and the read are one answer = `true` |
| The refusal vocabulary is still closed | `RefusalKind::COUNT` is **81**, `ALL.len() == COUNT`; **80 → 81** for this slice's one new name, cascaded into `tests/u5_measure.rs` by that file's own account |
| The table is honest about its size | `U6 tool table: 8 declared verbs, closed and bijective`; the parse-fault line names all eight, so an undeclared verb is still named against the **declared** list |

### What moved, and by how much

The captures are kept beside this record — `docs/baselines/u9-pre-stdout.txt`
(`sha256 b2363fe7acb7311f54570eb674824f8c40d1870b004b13772891e0e0db9362bc`) and
`docs/baselines/u9-post-stdout.txt` (`sha256 407080a854847a4dc207a71c4459c89831643263c32cc2724797b800c7dd42fc`),
with `u9-pre-stderr.txt` (`7cea9c2f…`, 806 bytes — the three meaningful lines plus a one-off build-cache
storage note, an environment artefact and not the program's) and `u9-post-stderr.txt` (`dbf5a759…`, 177
bytes, the three lines alone, the form §27 recorded). **The pre capture is byte-identical to
`u8-post-stdout.txt`** — the pass began exactly where §27's fix pass ended, which is why the diff below is
the pass's own movement and not a re-baseline.

`cargo run --offline` moved **467 → 487** lines of stdout: **26 lines added** and **6 rewritten in place**
(six lines removed, each replaced rather than dropped), for a net **+20**. Accounted line by line:

| Lines | What | Why |
|---|---|---|
| +4 (`179a180,183`) | the two rows' declarations in the table print — verb, arguments, owner, `record false`, checks, and the sentence each row answers with | `save`/`load` are the seventh and eighth rows |
| +13 (`215a220,232`) | the two **served calls** — `U6 call \`save path=u6-world.save\`` and `U6 call \`load … digest=611aadcb…\``, each with its answer, its four checks, its claims line, its effect (`work 0/0 columns`, `held 1 -> 1`, `record 2 -> 2 events, appended nothing`) and its verification — plus the `U6 document rows:` summary line | the rows are driven through the served line, as every other row is |
| +4, −1 (`277c294,297`) | the old `U9 owed:` line (*the document form … and `save`/`load` as rows of the one tool table*) became **three document lines** in its place — `save`, `load`, and the three refusals — plus the one new `U9 owed:` line, so nothing is dropped without its replacement | the owed capability is now a measured one; what remains owed is named where it moved |
| 1-for-1 (`167c167`) | `U6 tool table: 6 declared verbs` → `8` | the table is closed at eight |
| 1-for-1 (`229c246`) | the parse-fault line's declared verb list: `…, admit` → `…, admit, save, load` | the refusal names the declared list, so the list moved with the table |
| 1-for-1 (`41c41`) | the `[U6]` registry line: *a closed six-row table* → *a closed **eight-row** table*, and its owed list loses *`save`/`load` as table rows* | the registry is the authority on what exists |
| 1-for-1 (`62c62`) | the `[U9]` registry line: *Owed: the document form … and `save`/`load` as rows* → the document form landed, owed now *a save of a resident set, compression, and a versioned migration path* | the same |
| 1-for-1 (`67c67`) | the `[U9]` `interface:` field gains `Document::write / Save::read_document, which reads only through U6's \`Intake::read\`` | the interface moved, so the field moved |

**Nothing else in the binary's output moved.** No other system's measurements, no other row's answer, no
digest of any measured thing: U1's band count, U2's identity/cache, U3's stage digest, U4's merge factor,
U5's band table and mass, U6's remaining gate lines (every one beyond the two in the table above — the
`truth`/`fields`/`geometry`/`held`/`record` rows, the two-refusal correspondence and the intake-by-reference
lines), U7's accuracies, U8's frame digest and constant census, and U10's tree all print exactly the bytes
the pre capture holds.

### The guards, proved to bite by name

Each of this slice's new guards was perturbed **one value at a time**, run through the product's own
evidence, and the touched file restored from a byte-exact copy and verified by digest. The helpers and the
raw runs are kept in `docs/baselines/u9-guards/` (`run-guards.sh`, `digests.txt`, and the three `.out`/
`.err` pairs). Every perturbed run **failed the slice's own bar** (`cargo run --offline` exit **1**, with
`Clean room FAILED: … U9=false`):

| Perturbation | Where | What the product then said | File digest, before = after |
|---|---|---|---|
| **a seed** — the encoder writes each per-domain stream seed as `stream_seed(domain) ^ 1` | `src/save/mod.rs` | `U9 could not load its own save: 5 refusal(s): identity_mismatch: the save's terrain stream seed is 110187191988996983 and this world's is 110187191988996982; …` — all **five** domains named, both values, and the same refusal reached the served `load` row | `594e4490…` |
| **a save digest** — the gate's own digest comparison neutered (`if found != source.digest()` → `if false`) | `src/intake.rs` | the moved document is no longer named `source_digest_mismatch`: `U9 document refusals: … a document that is not the one the reference names [**save_tampered**] — each by its own name = **false**`. The fault is still refused (U9's parser catches it) but at the **wrong level** — which is what the gate's check is for | `c1f3af54…` |
| **a record field** — the write row's answer carries `record_len: chain.len() + 1` | `src/bridge/gate.rs` | `U6 NOT VERIFIED: 1 refusal(s): receipt_mismatch: this receipt carries record length = 3 and the document and the world establish record length = 2`, and `U9 document: … its receipt verifies … = false` | `8cb4a216…` |

Every one of the three files was restored byte-exact: the digests before and after are equal, and all four
files this pass perturbs are pinned in `u9-guards/digests.txt`. Nothing was left perturbed, and the final
captures were taken **after** the last restore.

### Measured on the final bytes

```bash
cd clean-room
cargo test --offline                                    # exit 0 — 338 passed / 0 failed
cargo clippy --offline --all-targets -- -D warnings      # exit 0 — no diagnostics
cargo run --offline                                      # exit 0 — 487 lines of stdout
cmp <(cargo run --offline 2>/dev/null) ../docs/baselines/u9-post-stdout.txt   # identical
```

**338 passed / 0 failed** = **276** library + **62** across the ten integration suites (U4 `2`, U5 `6`,
U6 `5 + 7 + 6 + 6`, U7 `17`, U9 `6`, U10 `7`); the library moved **274 → 276** and `tests/u9_saves.rs`
**4 → 6** — the two new tests read a save back **over the served line** and through the **content gate**
(`a_world_is_written_and_read_back_over_the_served_line`,
`a_save_document_is_written_at_a_reference_and_read_back_through_the_content_gate`). Digests of the files
this pass wrote or moved: `src/save/document.rs` `50c8e133…`, `src/save/mod.rs` `594e4490…`,
`src/bridge/table.rs` `cd0e8c76…`, `src/bridge/receipt.rs` `a7a87c40…`, `src/bridge/gate.rs` `8cb4a216…`,
`src/bridge/wire.rs` `830460e1…`, `src/bridge/mod.rs` `b619bd0c…`, `src/intake.rs` `c1f3af54…`,
`src/refusal.rs` `98d22d4c…`, `src/evidence.rs` `5c5cbc8a…`, `src/systems.rs` `a77effa5…`,
`src/reference.rs` `2956a0c1…`, `src/save/tests.rs` `0e82adec…`, `src/bridge/tests.rs` `3477b3b8…`,
`tests/u9_saves.rs` `e980b52e…`, `tests/u5_measure.rs` `d6643649…`, `tests/u6_bridge.rs` `9076fb3e…`,
`README.md` `39bf1c62…`; `src/lib.rs` is unchanged (`dc92774e…`).

**The record cascade, to measured values.** Every figure and row this slice moved was corrected where it
stood rather than left to read as a claim: §5's row 5 (U6's remainder loses *`save`/`load` as rows*) and
row 9 (U9's owed list) with dated corrections; **§16.2**'s row 9, whose *adjacent, not blocking* column
predicted this exact outcome; `README.md` (the test counts, the stdout line count, and U9's own paragraph —
*built in both of its forms*); `specs/006-world-saves-and-seeds/` `spec.md`'s two assumptions, `plan.md`'s
*What this slice does not land*, `tasks.md`'s owed list, `contracts/u9-seams.md`'s C4 (the two bullets it
records as owed now carry what landed), `quickstart.md` (a new Scenario 4 for the document form, the
renumbered gate scenario, the moved `U9 owed:` line) and `checklists/requirements.md`; and
`specs/005-u6-record-api/` `plan.md`'s *Scale/Scope*, `contracts/u6-seams.md`'s `TOOLS` comment,
`research.md`'s D7, `tasks.md`'s T009/T039 and `checklists/requirements.md`'s scope row — each of which now
says the table is closed at **eight** and names where that happened. `src/systems.rs` carries the same two
corrections as values the binary prints.

### What did not move

No entry in §2, no other row verdict, no citation: §2's thirteen entries are untouched (U9's `status:`
string is the entry's own field and moved with the system, which is what a registry is for), and U11–U13
still read `absent`. **U1's remainder**, **U6's remainder** (the document record and its compression, the
structural judge, the ballot, the policy matrix, the recipe, the API document, the legacy-vocabulary
bijection), **U10's four rulings**, **`mc-ar-mod`**, the **`noul`/`score` calibration sets** and the
**score instrument's rungs** stand **owed and untouched** — no file under `src/gameplay.rs`, `src/axis.rs`,
`src/laya.rs` or `src/von.rs` was edited by this pass. No second reader, parser or serializer appeared: the
document form reads through `Intake::read` and parses with `Save::parse`, and `save/document.rs` is the
clean room's only writer of a save document. Nothing under `crates/` was read or written (P7), and
`clean-room/` is still untracked in version control, so no VCS provenance is claimed for this slice
either.

### What remains owed after this slice

1. **A save of a resident set** (a save is the record, not the residency — F1), **compression**, and a
   **versioned migration path** — U9's own remainder, printed by the binary as `U9 owed:`.
2. **U6's remainder**, unchanged and named: the document record and its compression, the structural judge,
   the ballot, the policy matrix, the recipe, the API document, the legacy-vocabulary bijection.
3. The mission's other owed items, untouched here: **the four U10 rulings**, **`mc-ar-mod`**, the
   **`noul`/`score` calibration sets**, the **score instrument's rungs**, and §5 rows 8, 11–13.

The plan's order then continues at **§5 row 8** (the Lane A/B/C measurement on the clean room itself) or,
inside U9/U6, at the remainder above — the choice is the owner's, and this pass takes it no further.

### The four graded dimensions, stated plainly

| Dimension | Verdict |
|---|---|
| **Correctness** — a world written at a declared reference reads back as the same world | **graded and passed**: 465 bytes at `the-world.save`, the `load` row's truth digest is the live world's, and the write and the read are one value — one world reached two ways, in-process and across a boundary |
| **The named claim** — the gate is a **hard** prerequisite, and a fault carries its own name | **graded and passed**: the reader has no `fs::read` and goes through `Intake::read`; an absent document, a moved one and bytes altered inside the save's trailer refuse as `source_absent`, `source_digest_mismatch` and `save_tampered`, each recorded; and the guard proof shows that removing the gate's digest check *moves the name* rather than removing the refusal |
| **Determinism and no partial state** — the bar the plan sets | **graded and passed**: two runs of the binary are byte-identical at 487 lines, the two captures are kept and diffs are line-by-line, every refusal yields no world and no record, and both rows declare `record false` with effects of `0/0 columns` and nothing appended |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: every figure in this record is a printed value or a measured digest; the three guards were perturbed and restored byte-exact with the digests recorded; the owed list is named and printed rather than half-built; and nothing under `crates/` was read or written (P7) |

---

## 29. The eighteenth pass — U6's policy chokepoint: one decision function, and a denial that is recorded

**Which row, and why this one.** §28 closed with four candidates on the plan's own order: §5 **row 8**
(the Lane A/B/C measurement on the clean room itself), **rows 11–13** (U11 the planet, U12 civilian
organisations, U13 the shell completed with its `gholtzap/lintapple` gate) and **U6's remainder**. The
row chosen is U6's remainder — and inside it, the clause the mission names directly: the **policy
security system**. U6's remainder is the one candidate that is *workable now*: every one of rows 8 and
11–13 waits on something else that is still owed (U4's octree and a scale ruling; U2's civilian
vocabulary; U1's remainder and the unowned P8 gate), while the matrix's own prerequisites — the closed
table, the one dispatch site, the chain's one writer, the refusal vocabulary — are all built and
measured. It is also the clause U12 was told to wait on (§5 row 12, §16.2 row 12): landing it moves
U12 to **one** outstanding prerequisite, and it is the shape §L-5 says the console, the notification
panel and the Laya gate all need.

**The one new module, and the shape of the claim.** `clean-room/src/policy.rs` (**649** lines) holds
the chokepoint and nothing else. Deny by default is **structural** rather than a runtime hope: `Scope`
(3 declared) and `Action` (13 declared) are closed enumerations whose `ALL`/`COUNT`/`name()`/`parse()`
are pinned to each other, `permission_of(action)` is an exhaustive `match` with **no wildcard arm** — so
a missing matrix entry is `E0004`, a compile error, rather than a deny nobody notices — and an act or a
scope outside the vocabulary does not parse at all. Permission is a property of the **action**: the
matrix takes an `Action` and nothing else, so the same act has one class whichever door it is reached
through. Grants are **scoped and attributed**, refused **at construction**; a denial **quotes the
list**; and `check_matrix` refuses a row a reader could not understand.

**The one call site.** `Gate::call` consults `decide` **exactly once**, before it dispatches, with the
act taken from the row's own `Tool::action` — the table's rows carry their act, so a door cannot be
added without naming one and the gate never guesses. A denial is **recorded** through the chain's one
writer (the request and the refusal's **name**, never its detail — the rule C4 already states) and then
handed back. The reference session's actor and its three grants are declared **once** in
`src/reference.rs` (`reference::caller`), beside the identity and the ruleset, so the binary, its
evidence and its tests act as one actor; and a `Gate` **cannot be built without one**, for the same
reason it cannot be built without a ruleset.

**A reading rather than a transcription — stated plainly.** The legacy witness
`crates/voxel-package/src/policy.rs` is named in `docs/HANDOFF.md` as carrying **23** closed actions.
This build declares **13**, each derived from the properties §L-5 itself names (the reads and the checks
everything depends on; the acts that change what the world holds or where it is written; the two
unconditional denials; the one rejection) and each reached by a row that exists or by a ruling that
names it. The difference is a **reading of this tree's own vocabulary** rather than a copy of the
witness's, and nothing under `crates/` was read for it (P7).

### The measured numbers (from `clean-room/`)

| Claim | Measurement (printed by the binary) |
|---|---|
| The matrix is closed, counted and readable | `U6 policy: 13 act(s), closed (6 of them allowed — the reads and the checks everything depends on; 4 gated; 2 denied unconditionally; 1 a rejection), every row describes itself = true, and the classes account for every act = true` |
| Deny by default, at the vocabulary **and** the matrix | `everything`, `execute`, `*` and `all` are neither acts nor scopes; a missing matrix entry is a **compile error** — the `match` has no wildcard arm = `true` |
| Grants are scoped and attributed, refused at construction | a grant with no granter [`grant_unattributed`], a scope that is `everything` [`grant_unscoped`], a scope nobody declares [`grant_unscoped`], and a caller with no actor [`grant_unattributed`] — each refused at construction = `true` |
| The reference actor is served every gated row | `save` wrote **496** byte(s), `load` read them back and `geometry` appended — **6 of 6** calls answered = `true` |
| An actor holding nothing, at the one call site | its three reads answer = `true`; all **four** gated acts refuse `[policy_denied ×4]` = `true`, each one **recorded** (the record moved by **4 of 4**) = `true`, and the record still verifies = `true` |
| A refusal quotes the list | `reader` may not `append_event`: it requires a grant of the `record` scope and `reader` holds no scopes — the declared scopes are `record`, `content`, `world` |
| The unconditional denials hold for an actor holding all 3 scopes | `delete_content` [`policy_denied`], `write_external` [`policy_denied`], and `persist_material` a **rejection rather than a permission** [`never_store`] = `true` |
| The refusal vocabulary is still closed | `RefusalKind::COUNT` is **86**, `ALL.len() == COUNT`; **81 → 86** for this slice's five new names (`policy_denied`, `grant_unattributed`, `grant_unscoped`, `never_store`, `action_undescribed`), cascaded into `tests/u5_measure.rs` by that file's own account |
| The table is honest about its acts | `U6 tool table: 8 declared verbs, closed and bijective`; four of the eight are **gated** (`geometry`, `admit`, `save`, `load`) and each names its act on its own row |

### What moved, and by how much

The captures are kept beside this record — `docs/baselines/u6-policy-pre-stdout.txt`
(`sha256 407080a854847a4dc207a71c4459c89831643263c32cc2724797b800c7dd42fc`, **487** lines) and
`docs/baselines/u6-policy-post-stdout.txt`
(`sha256 1491a596a1b8f2584347e99fc4a65fbde112de8e4b52a6cc5a404adbacb58e6d`, **494** lines), with
`u6-policy-pre-stderr.txt` (`dbf5a759…`, 177 bytes) and `u6-policy-post-stderr.txt` (`0dd174c9…`, 177
bytes, the same three lines). **The pre capture is byte-identical to §28's post capture** — the pass
began exactly where the last one ended, which is why the diff below is this pass's own movement rather
than a re-baseline; the post capture was taken **after** the last guard restore and is the same bytes a
fresh `cargo run --offline` prints (`cmp` reports identical, twice).

`cargo run --offline` moved **487 → 494** lines of stdout: **7 lines added** and **3 rewritten in place**
(three lines replaced rather than dropped), for a net **+7**. Accounted line by line:

| Lines | What | Why |
|---|---|---|
| 1-for-1 (`41c41`) | the `[U6]` registry line gains the policy chokepoint and its owed list loses *the policy matrix* | the registry is the authority on what exists, and the matrix now exists |
| 1-for-1 (`46c46`) | the `[U6]` `interface:` field: `policy::decide` becomes the chokepoint's own names (`Action`, `Scope`, `Grant`, `Caller`, `PermissionClass`, `permission_of`, `decide`, `check_matrix` *consulted by the gate at one call site*), `bridge::TOOLS + Receipt` gains `Tool::action`, and `reference::gate` gains `caller` and `serve_stdio` | the interface moved, so the field moved |
| 1-for-1 (`83c83`) | the `[U12]` registry line: *waits on U2's vocabulary, U6's matrix and U7* becomes *waits on **one** thing — U2's owed civilian/government vocabulary* | the matrix landed here and U7 landed at §25 |
| +7, −1 (`276c276,283`) | the old `U6 still owed:` line became **seven policy lines** (the matrix and its classes; deny by default; the four construction refusals; the operator's six served calls; the reader's four recorded denials; the quoted refusal; the unconditional denials) plus the rewritten owed line, so nothing is dropped without its replacement | the capability is now measured, and what remains owed is named where it moved |

**Nothing else in the binary's output moved.** No other system's measurements, no other row's answer,
no digest of any measured thing: U1's band count, U2's identity and cache, U3's stage digest, U4's merge
factor, U5's band table and mass, every other U6 line (the `truth`/`fields`/`geometry`/`held`/`record`
rows, the two-refusal correspondence, the intake-by-reference lines, the two document rows), U7's
accuracies, U8's frame digest and constant census, U9's round trip and U10's tree all print exactly the
bytes the pre capture holds.

### The guards, proved to bite by name

Each of this slice's seven guards was perturbed **one value at a time**, run through the product's own
evidence and the full suite, and the touched file restored from a byte-exact copy and verified by
digest. The harness and its raw runs are kept in `docs/baselines/u6-guards/` (`run-guards.sh`,
`digests.txt`, `proof.txt`, and one `.out`/`.err`/`.test.log` triple per case). Every perturbed run
**failed the slice's own bar** — `cargo run --offline` exit **1**, stderr printing
`Clean room FAILED: a measured bar is not met (… U6=false …)` — and the test suite exit **101**, each
with the test that names the guard failing:

| Perturbation (subject) | Where | What the product then said | Test that failed | File digest, before = after |
|---|---|---|---|---|
| **the matrix's class** — `append_event` declared `AlwaysAllowed` | `src/policy.rs` 336 | `the reader was served \`geometry chunk=1,0,0 level=0\` — the chokepoint did not bite`, and the matrix line reads `7 of them allowed … 3 gated` | `the_ungated_acts_are_the_ones_the_documents_say_are_ungated` | `17d9e26a…` |
| **the act is the row's own declaration** — `geometry`'s row declares `read_world` | `src/bridge/table.rs` 331 | the same served line: the row now reaches an act nobody denies | `a_class_is_a_property_of_the_act_and_not_of_the_door`, `every_declared_door_names_one_act_and_every_act_is_readable`, `a_caller_with_no_grants_is_refused_by_name_and_the_refusal_is_recorded` | `f779a3e1…` |
| **the one call site** — the gate consults the chokepoint about another act than the row declares | `src/bridge/gate.rs` 204 | the same served line (the same bar catches both subjects) | `a_caller_with_no_grants_is_refused_by_name_and_the_refusal_is_recorded` | `32640039…` |
| **a denial is recorded** — the denial path returns without writing the refusal | `src/bridge/gate.rs` 205 | `each one **recorded** (the record moved by 0 of 4) = false` | `a_caller_with_no_grants_is_refused_by_name_and_the_refusal_is_recorded` | `32640039…` |
| **grants refused construction under their own name** — no granter refused as `grant_unscoped` | `src/policy.rs` 128 | `a grant with no granter [grant_unscoped] … each refused at construction = false` | `policy::tests::a_grant_with_no_granter_or_no_scope_is_refused_at_construction` | `17d9e26a…` |
| **a rejection is not a denial** — `persist_material` declared `AlwaysDenied` | `src/policy.rs` 348 | `persist_material is a rejection rather than a permission [policy_denied] = false`, and the matrix line reads `3 denied unconditionally; 0 a rejection` | `deny_by_default_and_every_denial_quotes_the_list`, `the_ungated_acts_…` | `17d9e26a…` |
| **every row describes itself** — an act that describes nothing | `src/policy.rs` 313 | `every row describes itself = false` | `policy::tests::the_vocabularies_are_closed_and_counted` | `17d9e26a…` |

Every perturbed file was restored **byte-exact**: the digests before and after are equal in all seven
cases (recorded per case in `u6-guards/proof.txt`), the three files are pinned in
`u6-guards/digests.txt`, and the final verification below was run **after** the last restore. Nothing
was left perturbed.

### Measured on the final bytes

```bash
cd clean-room
cargo test --offline                                    # exit 0 — 351 passed / 0 failed
cargo clippy --offline --all-targets -- -D warnings      # exit 0 — no diagnostics
cargo run --offline                                      # exit 0 — 494 lines of stdout
cmp <(cargo run --offline 2>/dev/null) ../docs/baselines/u6-policy-post-stdout.txt   # identical
```

**351 passed / 0 failed** = **283** library + **68** across the ten integration suites (U4 `2`, U5 `6`,
U6 `5 + 7 + 6 + **6** + 6`, U7 `17`, U9 `6`, U10 `7`); the library moved **276 → 283** and the
integration suites **62 → 68** — the new `tests/u6_policy.rs` (six tests, through the public surface and
a running gate) and `src/policy.rs`'s seven unit tests. Digests of the files this pass wrote or moved:
`src/policy.rs` `17d9e26a…`, `src/refusal.rs` `630421ae…`, `src/lib.rs` `e4c2b93e…`,
`src/bridge/table.rs` `f779a3e1…`, `src/bridge/gate.rs` `32640039…`, `src/bridge/mod.rs` `e30e5e4c…`,
`src/reference.rs` `fd7b61d4…`, `src/evidence.rs` `382c879b…`, `src/systems.rs` `ccd782b9…`,
`tests/u6_policy.rs` `9bebe86d…`, `tests/u5_measure.rs` `b3d7761c…`, `README.md` `4568e5ec…`, and the
specification files whose own pasted or promised figures this pass moved
(`specs/005-u6-record-api/{spec,plan,tasks,research,quickstart}.md`, its `contracts/u6-seams.md` and
`checklists/requirements.md`, `specs/006-world-saves-and-seeds/spec.md` and `quickstart.md`, and
`specs/007-u10-gameplay-tree/{spec,plan}.md`).

**The record cascade, to measured values.** Every figure and row this slice moved was corrected where
it stood rather than left to read as a claim: §5's row 5 (U6's remainder loses the policy matrix, and
its own remainder is named in its place) and row 12 (U12's check is half-measured, and the row waits on
one thing); §16's U12 section and §16.2's row 12 (the matrix off the prerequisite list, U7 marked
built); `clean-room/README.md` (the test counts, the stdout line count, U6's own paragraph);
`src/systems.rs`'s U6 and U12 entries (the binary's own registry, so the two corrections are *printed*);
`clean-room/specs/005-u6-record-api/` — `spec.md`'s owed assumption, `plan.md`'s *What this slice does
not land*, `tasks.md`'s owed list plus **Phase 11** (T045–T048), `research.md`'s D7 and its open
questions, `quickstart.md` (a new **Scenario 8** with the pasted lines, and its owed list),
`contracts/u6-seams.md` (**C5**, the policy seam, and `Tool`'s `action` field in C1),
`checklists/requirements.md` (the scope row and the chokepoint review); and
`clean-room/specs/006-world-saves-and-seeds/spec.md`/`quickstart.md` (the second's §28 pasted run now
carries this pass's own numbers beside it) and `clean-room/specs/007-u10-gameplay-tree/`
`spec.md`/`plan.md`, each of which had named the matrix owed. No entry was added to `docs/LEDGER.md`:
§28 added none, and no figure a C-entry carries is moved by this slice.

### What did not move

No entry in §2, no other row verdict, no citation: §2's thirteen entries are untouched (U6's own
`unifies` line already names *the permission policy matrix* as something U6 unifies, which is now also
true of the code), and U11, U12 and U13 still read `absent` except for the one registry sentence each
that a landed prerequisite corrects. **U1's remainder**, **U6's remainder** (the document record and its
compression, the structural judge, the ballot, the recipe format, the API document, the crate-by-axis
census, a pin over a request line, the legacy-vocabulary bijection — and, inside the matrix, the
self-describing protocol and the tool-surface narrowing over a mod-supplied tool list), **U9's own
remainder**, **U10's four rulings**, **`mc-ar-mod`**, the **`noul`/`score` calibration sets** and the
**score instrument's rungs** stand **owed and untouched** — no file under `src/gameplay.rs`,
`src/axis.rs`, `src/laya.rs` or `src/von.rs` was edited by this pass. No second reader, parser, renderer
or serializer appeared, and no second action vocabulary: the act lives on the row, the class in one
matrix, and the decision in one function. Nothing under `crates/` was read or written (P7), and
`clean-room/` is still untracked in version control, so no VCS provenance is claimed for this slice
either.

### What remains owed after this slice

1. **The matrix's own remainder**, named rather than implied: the **self-describing protocol** an
autonomous actor would discover the verbs through, and the **tool-surface narrowing** over a
mod-supplied tool list (L-5's *"a prompt-injected model cannot exfiltrate through a tool that does not
exist"* — the declared list is exhaustive and every row names one act today, but a *mod-supplied* list
has no caller in this build).
2. **U6's remainder**, unchanged otherwise and printed: the document record and its compression, the
   structural judge, the ballot, the recipe format, the API document, the crate-by-axis census, a pin
   over a request line, the legacy-vocabulary bijection.
3. The mission's other owed items, untouched here: **the four U10 rulings**, **`mc-ar-mod`**, the
   **`noul`/`score` calibration sets**, the **score instrument's rungs**, **U1's remainder**, and §5 rows
   **8** and **11–13**.

The plan's order then continues at **§5 row 8** (the Lane A/B/C measurement on the clean room itself) or,
where a prerequisite now exists, at **row 12** — U12 is one owed vocabulary away from startable, and this
pass moved it there — or inside U6 at the remainder above. The choice is the owner's, and this pass takes
it no further.

### The four graded dimensions, stated plainly

| Dimension | Verdict |
|---|---|
| **Correctness** — an act a caller may not perform is refused, and the refusal is a fact on the record | **graded and passed**: the reference operator is served **6 of 6** gated calls; an actor holding nothing is refused `policy_denied` on all four gated rows **before they dispatch**, each denial recorded (the record moved by 4 of 4) with the record still verifying; a denial names the act, the scope it needs, what the caller holds with who granted each, and the declared scopes |
| **The named claim** — the chokepoint is *one* function, and deny by default is structural | **graded and passed**: `permission_of` takes an `Action` and nothing else and has **no wildcard arm**, so a missing entry is `E0004`; an act or a scope outside the vocabulary does not parse; the gate consults the decision function at exactly one call site with the act from the row itself — and the guard proof shows that perturbing **either** the row's declaration **or** the call site is caught by the same bar |
| **Determinism and no partial state** — the bar the plan sets | **graded and passed**: two runs of the binary are byte-identical at 494 lines, the captures are kept and the diff is line-by-line, a gated act with no grant writes **no** answer and one recorded refusal, and an unauthorised grant is a value the build **cannot hold** rather than one it tolerates |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: every figure in this record is a printed value or a measured digest; seven guards were perturbed one value at a time, observed to fail the bar by name, and restored byte-exact with the digests recorded; the vocabulary's own size (**13** against the witness's 23) is stated as a reading rather than hidden; the owed list is named and printed; and nothing under `crates/` was read or written (P7) |
## 30. The nineteenth pass — U5's voices: the feeling wheel read as a value, and one bounded buffer (2026-10-12)

The mission names procedural audio, music and ambient systems **driven by the feeling wheel**, and §24's own
owed list put *the audio's own voices and the music/ambient beds* there. This pass lands the **voices** and
the wheel they are driven by, on the seam §24 built rather than beside it, and leaves the music layer, the
material's own voice and wind owed **by name**.

### What landed

* **`src/feeling.rs` (new)** — the wheel as a **value**: six families (`Mad` red 645 nm, `Scared` orange
  600, `Joyful` yellow 575, `Powerful` green 545, `Peaceful` blue 480, `Sad` purple 420 — *"each anchor is
  a real wavelength, so a state is a spectral read and not only a word"*), the integer radius with the
  record's own marks (`resolved 0`, `calm 13107`, `present 32767`, `strong 49151`, `extreme 65535`), the
  signed **drift** with the excursion the record defines (*"the reading's excursion is the radius' fraction
  of it"*), the **signature** (one declared band wide at the centre, all twelve at the rim), and the drift's
  **ladder**.
* **`src/light.rs`** — one new method: `Spectra::band_of_nm`, so *which band holds this wavelength* has
  **one owner** (the table's own span arithmetic) rather than a second copy inside the wheel. The interval
  is half-open because two anchors (600 nm and 575 nm) land exactly on a shared edge, and a wavelength
  outside the table's span is `None` — a refusal, never the nearest band.
* **`src/audio.rs`** — the **voices**: `voice(feeling, weather, noise, frames) -> Result<Voice, Refusal>`,
  one buffer carrying **two voices** (the bed the wheel drives and the rain the onset field drives) with its
  own measurements on the type — frames, one channel, both levels, the signature, the twelve tones, the peak,
  the RMS and the digest of the samples' own bytes. The samples are bounded **by the mixer's two declared
  ceilings** (`BED_GAIN + RAIN_GAIN`), and the binary measures that rather than assuming it.
* **`src/refusal.rs`** — four names, one per subject: `feeling_absent` (no reading at all),
  `feeling_undeclared` (a word the wheel does not declare), `feeling_uncalibrated` (a radius past the rim,
  or a drift that walks the peak outside the twelve declared bands) and `feeling_contradictory` (more than
  one family at once — *"select one dominant family rather than blending hues"*). **86 → 90**,
  `ALL.len() == COUNT`, cascaded into `tests/u5_measure.rs` by that file's own account.
* **`src/evidence.rs`** — four lines in `check_u5` (`U5 voices`, `U5 rain`, `U5 buffer`, `U5 voices refused
  by name`) and `voices_ok` in the bar's own conjunction. The rain voice is rendered from **the field's own
  first wet cell**, found by a bounded scan rather than assumed, and a field with no wet cell fails by name.
* **`tests/u5_measure.rs`** — two new tests through the public surface: the bounded buffer with its
  **digest pinned as a literal**, and the four refusals by name.

### The record's own three contradictions, named

1. **The label count.** The wheel's two files disagree: `SKILL.md` — *"6 primaries + 72 secondaries + 432
   tertiaries = 510 labels"*; `reference.md` — *"6 primary emotions, 36 secondary emotions, and 36 tertiary
   emotions, for 78 named emotion labels total"*. Both agree on the six families, their colours and their
   anchors, which is all this build reads; the counts are **owed to whoever owns that document**.
2. **The width at a strong radius.** The worked example `irritated` says a *strong* radius with no drift is
   *"one band wide at red"*, while the geometry says *"at the centre the signature is one band wide … at the
   rim it fills all twelve declared bands"*. The prose is the centre's reading. This build follows the
   geometry — both ends of the width are fixed there — and the disagreement is a **test**, not a footnote.
3. **Which twelve bands.** The wheel names three of its twelve (*"ionizing, the visible, and radio"*) and
   does not list the rest. This build reads the wheel's twelve as **the tree's own twelve** (the light
   table's published span) instead of inventing nine names, and the consequence is the refusal that matters
   most: the wheel's own third example, `carrying-the-loss` — Sad at the extreme radius with drift −4, whose
   peak *"walks out of the visible into the ultra-violet"* — **cannot be voiced by this table** (purple sits
   in its first band, and there is no band below 400 nm), so it is refused as `feeling_uncalibrated` and the
   refusal quotes the ladder it does have.

### The movement, measured

| Quantity | Before | After |
|---|---|---|
| Suite total | 351 passed / 0 failed | **365 passed / 0 failed** (+14) |
| Library tests | 283 | **295** (+12: `feeling` 7, `audio` 4, `light` 1) |
| `tests/u5_measure.rs` | 6 | **8** (+2) |
| `RefusalKind`'s closed vocabulary | 86 | **90** (`feeling_absent`, `feeling_undeclared`, `feeling_uncalibrated`, `feeling_contradictory`) |
| `cargo run --offline` stdout | 494 lines (`1491a596…`) | **498** (`a4fb60fb…`): four added, **none removed and none rewritten** |
| The wheel's own reading, as the binary prints it | — | `powerful`/green/545 nm, radius 49151, drift −1 → the peak in band 4 of 12, 12 bands sounding, ladder −5..=6, the bed at 0.3750 of 0.5 |
| The rain voice | — | the field's first wet cell at t7000 is (0, 1) = 0.0711 → gain 0.1624 → **0.0812 of 0.5** |
| The buffer | — | 2 048 frames at 48 000 Hz, 1 channel, 110.00–739.99 Hz, peak **0.337501**, rms **0.093172**, inside the 1.0 ceiling `(bounded)`, digest `f5a59a8b` |
| The pinned voice (the test's own fixture, a dry column) | — | digest `a33ede013edf0e2f97acfe33bece2af5a9b78d9583ba89972a79c33ed1f754a2` |

**Correction (2026-10-12, §31).** The movement pair above describes the build **before this pass's own last
edit** to `src/systems.rs` — the registry row that named the voices, edited at 14:33; the capture was taken
at 14:30 — so its `a4fb60fb…` and its "four added, **none removed and none rewritten**" are true of
pre → post and false of pre → final, which is **3 rewritten + 4 added** (the `[U5]` row, its `derived from:`
line and its `interface:` line; digest `b8203cbc…`). The final pair is `final-stdout.txt` /
`final-stderr.txt`, and the line-by-line account is §31.

### The guard proof

Seven guards, each perturbed **in its own subject** and each observed to fail the bar by name, with every
touched file restored and its digest checked before the next case: `feeling_absent`'s own name;
`feeling_undeclared`'s own name; the **ladder** (one band wider accepts the wheel's own out-of-visible
example); `feeling_contradictory`'s own name; the **normalisation** the levels are bounded by; the
**band-tone calibration** the digest is pinned to; and the table's own half-open band reading. The harness
and its output are kept: [`run-guards.sh`](baselines/u5-voices/run-guards.sh) and its
[`proof.txt`](baselines/u5-voices/proof.txt). The four source digests are **identical before and after** the
harness ran, which is the restore proof. This harness answers the two quality defects §29's own was graded
on: it restores through a `trap`, so an interrupt cannot leave the tree perturbed, and it anchors on the
*value* it perturbs rather than on a line number.

### An incident, recorded rather than hidden

A write to `clean-room/src/light.rs` **truncated the file to zero bytes** before it failed (a tool-level
failure on a path given with backslashes; the file is untracked, so git could not restore it). It was
recovered from a backup taken earlier the same day (`%TEMP%/qa1/backup/light.rs`, 909 lines), and the
recovery is **verified behaviourally**: with the restored bytes the binary's stdout is **byte-identical** to
the capture taken before the edit (`1491a596a1b8f2584347e99fc4a65fbde112de8e4b52a6cc5a404adbacb58e6d`, 494
lines, `cargo run` exit 0) and the suite is green — so no measurement in this record rests on the restored
file alone. Every file edited after that point was snapshotted first, and every edit was size-checked.

### The four graded dimensions, stated plainly

| Dimension | Verdict |
|---|---|
| **Correctness** — a state the record cannot voice is refused, and the refusal is the record's own rule | **graded and passed**: four refusals, four subjects, each printed `true` by the binary and each asserted by a test; the wheel's own `carrying-the-loss` is refused because the tree's table has no band below 400 nm; the buffer is measured, not promised (`peak 0.337501` inside the 1.0 ceiling) |
| **The named claim** — the wheel drives the sound, and the seam is §24's rather than a second path | **graded and passed**: `onset`/`Weather::gain` is the one weather read, `Feeling::signature` is the one shape, `light`'s table is the one band table (so the wheel's twelve *are* the tree's twelve), and the bed's level is the radius' own fraction of its ceiling — one concern, one owner each |
| **Determinism and no partial state** — the plan's bar | **graded and passed**: two runs of the binary are byte-identical (`a4fb60fb…`), a shorter render is a prefix of a longer one, the same inputs are the same digest, and both captures are kept so the four added lines and the zero rewritten ones are readable |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: every figure here is a printed value or a measured digest; the wheel's three self-contradictions are quoted; the label count, the sampled hexes and the locus sweep are owed **by name**; the write incident is recorded; and nothing under `crates/` was read or written (P7) |

### Owed, and named

The **music layer** the mission's own sentence names (*"procedural audio, music and ambient systems"*); the
**material's** own voice (rain on basalt is not rain on water — that needs the acoustic columns the roster's
ruling puts on the material row, H-20.4); **wind** (it needs a surface geometry and a pressure field this
build does not carry); the wheel's **label counts** (510 against 78) and its **sampled hexes**; the **locus
sweep** (*"so a family's later states lean toward the next family's hue"* — an angle effect, and this build
carries no angle); and, unchanged, §24's own remainder (ozone, an azimuth, the per-cell frame format), §29's
remainder, the four U10 rulings, `mc-ar-mod`, the `noul`/`score` calibration sets, U1's remainder, and §5
rows **8** and **11–13**. **This pass asked the user nothing**: the mission's own instruction was that the
voices need no ruling, so the owed items above are recorded rather than put as questions.

The next buildable step with its reconnaissance already done is the voices' own next piece — the
**material's own voice** (rain per material, from the roster's acoustic columns) and then the **music layer**
— or, at the owner's choice, §5's **row 8**.

## 31. The record repair — the movement re-measured against the final bytes, and the wheel's own documents named (2026-10-12)

The audit of §30 found the behaviour clean and the record wrong. This pass repairs the record and **changes
no Rust**: all three defects are claims, and where the smallest correct fix would have touched a source
comment (the wheel's provenance note in `src/feeling.rs`), the citation was left byte-identical and the
finding is carried here and in the owed lists instead — a doc-comment edit would have invalidated the guard
harness's recorded digests for `feeling.rs` for no behavioural gain, and the harness and its proof are left
alone. Three defects, three repairs.

### 1. The movement was accounted against a superseded build

§30's `post-*` pair (14:30:03) was taken **before its own last edit** to `src/systems.rs` (14:33:57) — the
edit that named the voices in the registry row — so the pair, its digests and §30's movement table describe
a build that no longer exists on disk, and the table's "none rewritten" is true of pre → post and **false of
pre → final**. Measured by diff: pre → final is **3 rewritten + 4 added** (7 new lines, 3 removed; net +4),
the three rewritten being the registry's own `[U5]` row, its `derived from:` line and its `interface:` line.
The final capture is landed beside the pre capture and taken **directly from the built binary**, so its
stderr carries the program's own line and no cargo timing — which the pre pair did not:

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), 494 lines | `1491a596…` | `7cea9c2f…` |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **498 lines** | **`b8203cbc…`** | **`f583ca18…`** |
| superseded (`post-stdout.txt` / `post-stderr.txt`), 498 lines | `a4fb60fb…` | `a3c3d1b6…` (the pre-rewrite build) |

The superseded pair is kept byte-for-byte, and its figures live only in §30's dated correction and in
`docs/baselines/u5-voices/digests.txt`, which now records the final pair and re-hashes the sources
(`src/systems.rs` was the one stale row: `ccd782b9…` → `5f399654…`). Two direct runs of the final binary
are byte-identical (`b8203cbc…` twice).

### 2. The cascade stopped short

Each stale figure of the named kinds — test totals, stdout line counts, kind counts — was brought to its
measured value or moved into this dated account, file by file:

| Where | Was | Now |
|---|---|---|
| `clean-room/README.md`, the run block | "added four and rewrote none; pre and post captures" | added four, rewrote three (each named); pre and **final** captures |
| `specs/008-…/spec.md` | 260 / 457 | **365 / 498** (with §24's own reading dated) |
| `specs/008-…/plan.md` | 260 / 457; `check_u5` "thirteen lines" | §24's reading dated; 365 / 498 and seventeen lines named |
| `specs/008-…/quickstart.md` | 260 / 457; "thirteen of them U5's" | **365 / 498**; **seventeen**; a §31 paragraph carries the voices' own movement, the old table kept as §24's dated record |
| `specs/008-…/tasks.md` | 457; thirteen lines | §24's reading dated, the voices' movement named |
| `specs/008-…/checklists/requirements.md` | 260 / 457; voices owed | **365 / 498**; the beds owed, the voices landed |
| the plan's §5 row 5 | "65 kinds … 457 lines" | a dated correction in the row: 86 → **90**, suite **365 / 0**, binary **498** |

The plan's own dated pass sections (§24, §26, §28, §29) keep their historical figures — they are the dated
account, and this section points at them; the LEDGER's **C-186** cascade paragraph, which carried `260` and
`457` as its own measurement, gains the same dated correction. The other slice records — `specs/005`'s §29-era movement to `494`, `specs/009`'s derivation from §24's `457`, `specs/010`'s shell pass — keep their own dated figures, and none of them presents them as the current tree.

### 3. The wheel's own documents live outside the tree

`src/feeling.rs`'s module doc cites `feeling-wheel/SKILL.md` and its sibling `reference.md` as the module's
source, and `src/systems.rs` lists them in `derived_from` — but neither file is in the repository (`find`
returns nothing, and the derivation-documents test in `tests/u5_measure.rs` checks every *other* module's
source document and cannot check these): the wheel is an **account skill** living in the author's own skill
directory outside this tree. Reading taken, and it is the smallest honest one: the module's citation names
its source, the source is outside the repository, and a reader of the tree cannot check it — recorded here
and **owed**: either the two documents are vendored into the tree (the owner's call — they are a skill's own
text) or a ruling says the provenance stands outside. **No second copy of the wheel's own tables was written
into the tree**, and no Rust was changed.

### The movement, as it stands on the final bytes

| Quantity | Value (measured 2026-10-12) |
|---|---|
| `cargo test --offline` | **365 passed / 0 failed** (295 lib + 70 integration) |
| `cargo clippy --offline --all-targets -- -D warnings` | exit **0** |
| `cargo run --offline` | exit **0**; stdout 498 lines, `b8203cbc…`; two runs identical |
| The binary's own stderr (direct run) | `f583ca18…`, the one `serve:` line |
| `RefusalKind`'s closed vocabulary | **90** |
| U5's own stdout lines | **17** (thirteen before the voices) |
| Sources | only `src/systems.rs` re-hashed (`5f399654…`); every other source digest unchanged |

### The four graded dimensions, stated plainly

| Dimension | Verdict |
|---|---|
| **Correctness** — the repair changed no behaviour, and every command was re-run on the final bytes | **graded and passed**: `cargo test --offline`, `cargo clippy --offline --all-targets -- -D warnings` and `cargo run --offline` all exit **0**; the capture pair is byte-checked (`cmp`) and two direct runs are byte-identical |
| **The named claim** — the movement now describes the artifact that exists | **graded and passed**: the diff is line-by-line (3 rewritten, 4 added, each named), the superseded digest is dated and kept, and §30's table carries the correction |
| **Determinism and no partial state** — the bar the plan sets | **graded and passed**: no file was left perturbed (only record files were edited), the harness and its four digests are untouched and still match the sources, and the final capture is reproducible |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: the stale figures are dated rather than deleted, the wheel's provenance is named owed rather than papered over, nothing under `crates/` was read or written, no Rust changed, and the stale-figure check is the grep, not memory |

### Owed, and named

The **music layer**; the **material's** own voice and **wind**; the wheel's **two source documents** (above);
the wheel's **label counts** (510 against 78) and its **sampled hexes**; the **locus sweep**; §24's own
remainder (ozone, an azimuth, the per-cell frame format); §29's remainder; the four U10 rulings;
`mc-ar-mod`; the `noul`/`score` calibration sets and the score instrument's rungs; U6's and U9's remainders;
U1's remainder; and §5 rows **8** and **11–13**. **This pass asked the user nothing and changed no
behaviour.**

The next buildable step is the voices' own next piece — the **material's own voice** (rain per material,
from the roster's acoustic columns) and then the **music layer** — or, at the owner's choice, §5's **row 8**.
## 32. U12's first mob — one being whose need the world's own cells meet, and every act a governed event (2026-10-13)

This pass built **the civilian system's first mob** — the fake autonomous player — as the row's own sentence
reads it. Reconnaissance first, and the record's own bar: **§5 row 12**'s check is *an organisation's member
acts only through the one tool table, and an act it may not do is refused by name and recorded*; **§16**'s
U12 entry names the interface (*an `Organisation` with declared membership and authority, and an actor that
asks the one tool table what it may do, and whose every act is a governed event on the one `Chain`*) and the
row's prerequisites (**U2's owed T016 vocabulary**, **U6's self-description**, **U7** — built, §25; the
**policy matrix** — landed, §29); and **§16.2 row 12** orders it after those two. The mission's own updated
component for this row adds three clauses: the being is **ensouled**, its **needs are met through the game's
own cellular automata**, and it is **driven by the existing laya/von integration** rather than a need tick.
**§33's repair (2026-10-13) flags the substitution this paragraph did not**: the clause is met by the world's
own cell-based water law as **one static evaluation at `Tick(0)`** — not by a stepping cellular automaton,
which is **owed** (§33 states it in the module header, in `check_u12`'s doc, in the acceptance instrument's
header and in the printed `U12 first mob:` line itself).
The record did not order a different unblocked row first — U9 landed (§17/§28), U10 is in progress with its
four rulings owed, U11 waits on U4's octree and a scale ruling, U13 is last by its own reasoning — so the row
was taken, with its two owed prerequisites read rather than invented.

### The readings this pass took rather than asking (each recorded as owed)

1. **The civilian nouns (§27's citation, and T016's own subject).** Two ladders in `axis.rs` are **owed by
   name**: `AxisKind::OrgRank` (an office rank) and `AxisKind::Certification` (the certification/resume bands),
   both owed to `docs/CIVILIAN-PORT.md` — which is an **inventory** of the civilians' process (`gov/mod.rs`,
   `resume.rs`) and **does not declare their tokens**. The smallest reading: **an office is the
   organisation's own word for it** (`tidewarden`), a name the organisation uses with the scopes its charter
   reaches, **never a fabricated rung**; the ladders stay owed and print owed. The restriction the record
   states (*a ladder that is nearly right reads as checked*) is kept, and `specs/002-world/tasks.md` T016 is
   annotated in place. **Recorded as owed**: the office rank's and the certification bands' tokens remain the
   user's to rule.
2. **The floor an act must reach.** The stage question's `passes` must be declared before any answer exists,
   so the organisation's **charter** declares it: `ACT_FLOOR = "verified-internal"` — an act is taken only
   where its own record has been read at the stage an *internal* act is at, because a member's act carries
   nobody's approval. The reading was **measured, not guessed** (a head declaring no approval of its own reads
   `verified-internal`; one declaring `approval: design-approved` reads `proposed`), and a **stricter
   organisation** exists as a declared value (`applied-internal`, "the draught guild") so the below-floor
   refusal is reachable rather than hypothetical.
3. **The world's own cells.** The clean room has one water truth: U3's `hydrology` stage (`elevation`,
   `water_level` — Priority-Flood's filled surface) and its flow routing (`d8_receivers`). The smallest
   reading: the being's world **is** that — read through the one field plane, walked along the receivers, with
   **no second needs model, scheduler or automaton**. Its need is a fold over its own acts (there is no need
   tick anywhere in the tree), and the refusal when the world cannot meet it is by name and recorded.

### What was built, one concern with one owner

| file | what it owns, alone |
|---|---|
| `src/being.rs` *(new)* | the being: `Organisation` (name, charter, office, scopes, floor), `Act`, `WorldRead` (the field-plane read, the depth, the flow step), `Being` (its record's claim as its soul check, its need as a fold, its cell read off its records, its reach, `tick`), `stage_of` (the laya/von seam), `draft_record`, `start_cell`/`dry_sink`, `chosen_stage`, the digests |
| `src/refusal.rs` | two new names, declared and tested: `water_unreachable`, `stage_below_floor` (90 → **92**) |
| `src/chain.rs` | one new event kind, declared and tested: `being_acted` (6 → **7**) — the record's owner is the chain, so a being's act is a governed event there rather than in a ledger of its own |
| `src/evidence.rs` | `check_u12`: the run the binary prints, and its bar |
| `src/systems.rs` | the `[U12]` registry row, rewritten from `absent` to the first slice built |
| `src/main.rs` | the wiring, and the failure line's own list |
| `src/lib.rs` *(unnamed in this table until §33's repair, 2026-10-13)* | the module list that publishes the new module — `pub mod being;`, without which nothing here is reachable |
| `tests/u12_being.rs` *(new)* | the row's check through the public API: the member's acts, the named-and-recorded denial, the three refusals, the owed vocabulary, two runs one digest |
| `tests/u5_measure.rs` *(unnamed in this table until §33's repair, 2026-10-13)* | the refusal count's own floor, moved 90 → **92** by this pass's two names, with the seventh `the same reason` note beside the assertion |
| `clean-room/README.md`, `specs/002-world/tasks.md`, `specs/006-world-saves-and-seeds/quickstart.md` | the figures this pass moved (365 → **374**, 498 → **504**), T016's annotation, and the one §29 note §31's own cascade table did not reach (kept dated, with §31–§32's values beside it) |

**Nothing under `crates/` was read or written** (P7), the library's only dependency is still `sha2`, and no
file outside `clean-room/` and `docs/` was touched by the build.

**Brought to the bytes by §33's repair (2026-10-13)**: this table listed `chosen_stage` among the things
`src/being.rs` owns, and §33 deleted it — a second argmax is gone, and the chosen option has one owner
(§33). The two rows above were added by §33 rather than left as a hole, because a table that omits two
files a pass changed is the record disagreeing with its own bytes.

### The capture before and after, and the movement accounted line by line

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), **498 lines** | `b8203cbc…` | `f583ca18…` (1 line) |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **504 lines** | **`6bf8c2d0…`** | **`f583ca18…`** (1 line, unchanged) |

The pre pair was taken from the built binary before the first edit, and it is byte-identical to §31's final
pair (`b8203cbc…`), so this pass began exactly where §31's repair ended. The movement by `diff` is **one line
rewritten and six added, none removed** (498 → 504, net +6), and each line is named:

* **rewritten, line 83** — the registry's own `[U12]` row: `absent (admitted by §16; … the policy chokepoint
  …)` → `**first slice built (2026-10-13, §32)**: **the first mob** (`src/being.rs`) …`;
* **added, lines 498–503** — the six `U12` evidence lines (organisation, first mob, stage question, governed
  act, refusals, determinism), inserted before the closing `Clean room OK:` line.

Every other line, digest and count in the binary's output is unchanged — including U7's classifier readings,
which the being's own question now asks beside the gate's demonstration. Two direct runs of the final binary
are **byte-identical** (`6bf8c2d0…` twice).

### The numbers, as the binary prints them

```
U12 organisation: `the undertow congregation`, charter `docs/CIVILIAN-PORT.md`, office `tidewarden` — its rank ladder stays owed by name (`AxisKind::OrgRank`); the charter reads all 13 declared action(s): 7 allowed, 6 refused by name
U12 first mob:    cell 14 of 2304 — 4 step(s) along the game's own flow law, then drank at cell 159 (0.010996 m of the world's own water); the need reads met = true; the one chain carries 5 event(s) (0 refusal(s)) and verifies = true
U12 stage question: reads `verified-internal` at the charter's floor `verified-internal` (origin the proof's own) over c957e3ab — the being's draft read in place, digest 979bf776; 1 declared feature(s) held
U12 governed act: `save_world` is refused `policy_denied` to this member, the refusal quotes the list = true, and it is on the chain = true
U12 refusals: a need the world cannot meet where the being may reach is refused `water_unreachable` and recorded = true (a dry sink) and = true (a reach spent on a cell the routing would carry); an act below the charter's own floor (applied-internal) is refused `stage_below_floor` and recorded = true; a being whose records cannot be read is refused `no_reading` = true
U12 determinism:  two runs of the first mob produce one chain digest = true (979bf776)
```

The being's birthplace (cell 14) is the **world's own answer**, not a coordinate chosen here: it is the first
dry cell whose flow path reaches standing water within the being's reach, found by `start_cell`. The walk is
14 → 62 → 110 → 158 → 159, the drink is at the cell the world's own filled surface stands over, and the
need's met/unmet reading is the fold over the being's own records rather than a counter beside it.

### The guards, nine, each proved to bite by name

[`run-guards.sh`](baselines/u12-first-mob/run-guards.sh) perturbs each new guard **in its own subject** and
runs the bar: the world's own water read; the game's own flow step; the being's reach; `water_unreachable`'s
own name; `stage_below_floor`'s own name; the charter's declared floor; the stage comparison; the recording
of a denial; and the unsouled refusal. All nine bit by name, every touched file came back **byte-exact**
(`sha256sum -c` against a snapshot taken before the first case, restored through a `trap`), and the harness's
own output is kept in [`proof.txt`](baselines/u12-first-mob/proof.txt) with each case's test log beside it.
The five source digests the harness restored to are recorded at its end and again in
[`source-digests.txt`](baselines/u12-first-mob/source-digests.txt).

### The done conditions, with exit statuses

| Check | Exit | Measured on the final bytes |
|---|---|---|
| `cargo test --offline` | **0** | **374 passed / 0 failed** across 14 suites (300 lib + 74 integration); 365 → 374 is this pass's +9 (the being's five unit tests and four integration tests) |
| `cargo clippy --offline --all-targets -- -D warnings` | **0** | no warnings |
| `cargo run --offline` | **0** | 504 stdout lines (`6bf8c2d0…`), 1 stderr line (`f583ca18…`), two runs byte-identical |
| the tree's own vocabulary | — | refusal names **90 → 92**, event kinds **6 → 7** (both declared, both tested, both named where they moved) |

### The four graded dimensions, stated plainly

| Dimension | Verdict |
|---|---|
| **Correctness** — the record's own rules refuse, and the refusal is the record's own | **graded and passed**: the member's charter read over all 13 declared actions (7 reached, 6 refused); an act it may not do refused `policy_denied` with the list quoted **and recorded**; a need the world cannot meet refused `water_unreachable` (both branches — the dry sink and the spent reach — proved separately); an act below its charter's floor refused `stage_below_floor`; a being without a claim refused `no_reading`, which is the reading layer's own word |
| **The named claim** — the mission's three clauses land on existing seams, not beside them | **graded and passed**: the being's choice is `laya`'s stage question over `von`'s five declared stages with its floor declared in the charter; its world is the one field plane's `elevation`/`water_level` and `d8_receivers`; its acts are `policy`'s one call site and `chain`'s one ledger; **no second needs model, scheduler or cellular path** — the need is `thirsty()`, a fold over the being's own records (asserted by its tests), the only clock in the module is the world's own `Tick`, and `being.rs` computes no state of its own between ticks |
| **Determinism and no partial state** — the plan's own bar | **graded and passed**: two runs of the first mob produce one chain digest (`979bf776…`); the binary's two runs are byte-identical; the being holds no mutable interior (its need, its cell and its steps are all reads off its chain), and a failed act is recorded rather than half-taken |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: every number here is a printed value, a measured digest or an exit status; the floor `verified-internal` is the **measured** reading of the being's own draft, and the stricter charter's `applied-internal` is how the below-floor case is reachable; the organisation's rank and certification ladders are **owed** by name and printed owed; the abbreviated digests above are the only elisions and each is expanded in the capture pair and in [`source-digests.txt`](baselines/u12-first-mob/source-digests.txt) |

### Owed, and named

**U6's self-describing protocol** (the verbs an autonomous actor discovers — this build's being *is* told by
its charter, which is not the protocol the row names); **U2's T016 vocabulary** — the office rank and the
certification bands, printed owed in `axis.rs`; **the other kinds of being** — a plant (a being that never
acts), an incarnate pilot, a god's office; **ensoulment's own slice as the record orders it** (soul-energy
accounting, the released essence, consent/rite/corruption — this pass read *von as the substrate the being is
read on*, which is the smallest reading, not the accounting); **relations and the court** (grudges, debts,
findings — a killing is not modelled here); **apprenticeship/tutoring**; **a population** (many beings, one
world, and the picture of it); and, unchanged, §24's remainder (ozone, an azimuth, the per-cell frame
format), §29's remainder, U1's remainder, the `noul`/`score` calibration sets, the **four U10 rulings**,
`mc-ar-mod`, and §5 rows **8**, **11** and **13**. **This pass asked the user nothing**: the mission's own
instruction was to take the smallest reading, record it as owed, and not stop.

### The next buildable step

The reconnaissance for the next step is this one's own remainder: **the second being** — one world with two
mobs, whose needs race for the same cells (the first place a *relation* between beings can be read rather
than asserted) — or the **self-describing protocol** the row names, which is what would let an autonomous
actor discover its verbs rather than be told them by its charter.
## 33. The record repair — the second argmax deleted, the two substitutions stated, and the nine guard logs brought to the bytes on disk (2026-10-13)

This pass **corrected the record rather than extending the tree**. A read-only audit of §32 named four
concrete defects; this pass closes exactly those and adds no capability. The library's only dependency is
still `sha2`; no refusal name, event kind, namespace or action moved (92 / 7 / 3 / 13, measured in the
sources and printed in the run); the being's drive, its clock and its chain are the same code §32 left; and
**nothing under `crates/` was read or written**.

### The four defects, and where each is closed

| the audit's finding | what this pass did |
|---|---|
| re-running [`run-guards.sh`](baselines/u12-first-mob/run-guards.sh) **rewrites** its nine captured `.test.log` files — each carries its own run's test-thread PIDs and cargo timings — so §32's original nine bytes no longer exist and their digests lived only in that session's transcript | the harness was re-run on the repaired bytes, and the nine files' **current** digests are recorded in [`baselines/u12-repair/digests.txt`](baselines/u12-repair/digests.txt) and in §32's own [`digests.txt`](baselines/u12-first-mob/digests.txt) under a dated §33 block. **Restoring §32's bytes was not possible and not honest**: nothing recomputes a PID. §32's own run's output is kept dated in [`baselines/u12-repair/proof.txt`](baselines/u12-repair/proof.txt) beside the re-run's, which is the run that produced the files on disk |
| §32 changed `src/lib.rs` and `tests/u5_measure.rs` and its own file table did not name them | both rows are in §32's table now — `src/lib.rs` (the module list that publishes `being`) and `tests/u5_measure.rs` (the refusal count's own floor, 90 → 92) — marked as §33's repair rather than left as a hole |
| the mission's clause *needs are met through the game's own cellular automata* was met by a **single static evaluation**, and the account did not flag the substitution | stated plainly here, in `being.rs`'s header, in `check_u12`'s own doc, in the acceptance instrument's header, **and in the printed run** — with a real stepping automaton on the field plane named **owed** (below). §32's own paragraph carries a dated §33 note now, so the substitution cannot be read past |
| `chosen_stage` was a second argmax over the evidence's scores — a second opinion about one reading, which could disagree with the instrument about a tie | **deleted**, mechanically: it was a `pub fn` with **one caller** (`stage_of`) and no test naming it. `laya::Answer::chosen()` is the chosen option's one owner, and `stage_of` now *reads* it. Its non-reading check was re-expressed in the typed vocabulary (`verdict() == Verdict::NoReading`) instead of comparing a name string |

### The two small honest fixes taken, and what was refused

1. **The floor's fact is declared, not inferred.** The being's draft head now carries `dependencies: []` — the
   key is **present** and the fact is read where it is written, rather than being inferred from a missing key —
   and the fixture in the being's own test declares it too. The run prints the fact's own **name**
   (`declares_no_dependency`) where it used to print a count, so what the floor stands on is in the output and
   not only in a doc comment. The existing stage-question test now pins both (the head declares the key; the
   reading's one feature is that fact) — **no new test, no count move**.
2. **The static read is stated where the run prints it.** The first-mob line says the walk is *one static
   evaluation of the roster at Tick(0), not a stepping cellular automaton — owed*.

**Refused, and named owed instead**: a stepping automaton on the field plane (`field_sampler`/`ProcessScheduler`
territory — a second scheduler path, not a repair), and a being-act declared-fact vocabulary (a second
vocabulary beside `von`'s twelve declared facts — the floor's one fact stays the patch record's, by absence,
until that vocabulary is ruled). Neither is glossed: both are named below. *(2026-10-13, §34: the stepping
automaton this sentence refused is **built** — `src/automaton.rs`, one state, one law and one tick, with its own
instrument and the run's `U12 stepped plane:` lines. The exclusion it was refused under is exactly what kept the
slice to one concern: no second scheduler, no clock and no thread. The being-act declared-fact vocabulary is
still owed.)*

### What moved, file by file

| file | what it owns after this pass |
|---|---|
| `src/being.rs` | the module's account of **what the world read is** (one static evaluation at `Tick(0)`; a stepping automaton owed) and of **what the floor rests on** (one declared fact, which holds by absence); `draft_record` and the design fixture declare `dependencies: []`; the stage-question test pins the declared key and the fact's name; `chosen_stage` deleted |
| `src/laya.rs` | the chosen option's **one owner**: `Answer::chosen()` reads the option the instrument itself scored. §33 drew the arrow here (digest `440c45ea…` → `abc47749…`), so `being.rs` has nothing to re-derive |
| `src/evidence.rs` | `check_u12`'s bar: the first-mob line states the static read, the stage line prints the declared fact's name, and `Drive` names the drive's return once — clippy's `type_complexity` was raised by the wider tuple, and a named type is the fix rather than an `allow` |
| `tests/u12_being.rs` | the same clause in the acceptance instrument's header, and its own fixture draft declares the key |
| `clean-room/README.md`, `specs/006-world-saves-and-seeds/quickstart.md` | this pass's capture pair and the §33 note beside §31–§32's figures |
| **nothing else** | `src/refusal.rs` (`25984677…`), `src/chain.rs` (`60d56a24…`), `src/systems.rs` (`72dc8ef5…`), `src/main.rs` (`951081f5…`), `src/lib.rs` (`8ca618a3…`) and `tests/u5_measure.rs` (`f9b19f45…`) are **byte-identical** to §32's recorded digests; no test was added or removed, and the systems registry's `[U12]` row still holds §32's sentence |

### The capture before and after, and the movement accounted line by line

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), **504 lines** | `6bf8c2d0…` | `f583ca18…` (1 line) |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **504 lines** | **`5f4d5c80…`** | **`f583ca18…`** (1 line, unchanged) |

The pre pair is **§32's own final pair**, so this pass began exactly where §32's account ended. The movement
by `diff` is **two lines rewritten, none added, none removed** (504 → 504), and both are named:

* **line 499** — `U12 first mob: … 4 step(s) along the game's own flow law` → `… along the game's own flow law
  (one static evaluation of the roster at Tick(0), not a stepping cellular automaton — owed) …`;
* **line 500** — `U12 stage question: … digest 979bf776; 1 declared feature(s) held` → `… digest 979bf776; the
  declared fact(s) that carried it: declares_no_dependency`, and the artifact's digest moved with the head it
  reads (`c957e3ab` → `460b9825`) because the draft now declares the key it is read for.

Every other line, digest and count in the binary's output is unchanged, the being's chain digest above all
(`979bf776` in the stage line and in the determinism line). Two direct runs of the final binary are
**byte-identical** (`5f4d5c80…` twice).

### The numbers, as the binary prints them

```
U12 organisation: `the undertow congregation`, charter `docs/CIVILIAN-PORT.md`, office `tidewarden` — its rank ladder stays owed by name (`AxisKind::OrgRank`); the charter reads all 13 declared action(s): 7 allowed, 6 refused by name
U12 first mob:    cell 14 of 2304 — 4 step(s) along the game's own flow law (one static evaluation of the roster at Tick(0), not a stepping cellular automaton — owed), then drank at cell 159 (0.010996 m of the world's own water); the need reads met = true; the one chain carries 5 event(s) (0 refusal(s)) and verifies = true
U12 stage question: reads `verified-internal` at the charter's floor `verified-internal` (origin the proof's own) over 460b9825 — the being's draft read in place, digest 979bf776; the declared fact(s) that carried it: declares_no_dependency
U12 governed act: `save_world` is refused `policy_denied` to this member, the refusal quotes the list = true, and it is on the chain = true
U12 refusals: a need the world cannot meet where the being may reach is refused `water_unreachable` and recorded = true (a dry sink) and = true (a reach spent on a cell the routing would carry); an act below the charter's own floor (applied-internal) is refused `stage_below_floor` and recorded = true; a being whose records cannot be read is refused `no_reading` = true
U12 determinism:  two runs of the first mob produce one chain digest = true (979bf776)
```

### The guards, nine, proved to bite on the repaired bytes

The harness was re-run unchanged, and it exited **0**: all nine cases bit **by name** — the world's own water
read, the game's own flow step, the being's reach, `water_unreachable`'s own name, `stage_below_floor`'s own
name, the charter's declared floor, the stage comparison, the recording of a denial, the unsouled refusal —
and every perturbed source came back **byte-exact** (`sha256sum -c` against the pre-run snapshot, restored
through a `trap`): `being.rs 613b1229…`, `refusal.rs 25984677…`, `chain.rs 60d56a24…`, `evidence.rs
1de78b22…`, `u12_being.rs ea8bc83a…`. Nine perturbations were first checked to match **exactly once** in the
edited sources, so no case could pass as a silent no-op.

The nine `.test.log` files beside §32's own proof are now **this run's**, and their digests — the values the
baseline was missing — are:

```
1cf2740f2290d9094058818ddc905be8889db1c696aff8898f4e5c274f2fada0  the_being's_own_reach.test.log
facd42c2c46629904892466f3b688e970ee0128c5bf1d22d8010ddd0f72ab91f  the_charter's_declared_floor.test.log
52e7dbdd1aaf95382ace8a82d162cd578333c2d3dbb7e4bfe16876947a4e6c87  the_denial_recorded.test.log
80a1427d33ff93718d5cdb081c16516d3921e73a89ce04dd992fc84ee168b875  the_game's_own_flow_step.test.log
9ffc40fc21763bcb90fb04f6172b18bc371e2cde1d0c2b2b3a685f67f5c6bf13  the_refusal's_own_name__stage_below_floor_.test.log
9efaa386af6eae3ff4706d508444aaa1c4bf9070374ebd325f9920ea5c0fc38c  the_refusal's_own_name__water_unreachable_.test.log
3fe71d091a39eb7cbb7ee89f9ce672ff9f107e3b2768413d348a8d01068cc225  the_stage_comparison.test.log
839f7f08823fb8697a2fd345f6b8402743315b52d4f114dbf4f673f6bd44d7ce  the_unsouled_refusal.test.log
b1f0d92bfa063d3e59d51725b6fe85ccd5e2e0207b06bcb092e55c26f9941a70  the_world's_own_water_read.test.log
```

§32's own run's output is kept in the same file, dated, because its panic **line numbers describe §32's
bytes** (`being.rs` was 907 lines then; the repaired file is 928).

### The done conditions, with exit statuses

| Check | Exit | Measured on the final bytes |
|---|---|---|
| `cargo test --offline` | **0** | **374 passed / 0 failed** across 14 suites (300 lib + 74 integration) — **unmoved** from §32, because this pass added no test and removed none |
| `cargo clippy --offline --all-targets -- -D warnings` | **0** | `Finished dev profile` — no diagnostics (one `type_complexity` was raised by the wider `drive` return and answered with a named type, not an `allow`) |
| `cargo run --offline` | **0** | 504 stdout lines (`5f4d5c80…`), two runs byte-identical, and the **binary's own** stderr is the one line `f583ca18…` (re-run directly to check: identical). `cargo run`'s stderr additionally carries cargo's own chatter — the kache layout advice, `Finished`, `Running` — which no capture of the binary contains |
| the tree's own vocabulary | — | refusal names **92**, event kinds **7**, namespaces **3**, actions **13** — all **unmoved** (`refusal.rs:328/424`, `chain.rs:59/99`, the run's own `13 declared action(s)`) |
| the guard harness | **0** | nine cases bit by name; five sources restored byte-exact (digests above) |

### The four graded dimensions, with the top gap in each

| Dimension | Verdict, and the top gap |
|---|---|
| **Correctness** — the record's own rules refuse, and the refusal is the record's own | **graded and passed**: 374/0 with the being's nine tests among them, and all nine guards bit by name on the repaired bytes — the drive, both `water_unreachable` branches and the below-floor path are unchanged by the deletion and the declared key. **Top gap**: every guard here proves the *being's* path bites; nothing in this slice exercises the world meeting a need it *can* meet for a being that cannot reach it — that is the second mob's territory, and it is owed. |
| **The named claim** — the mission's three clauses land on existing seams, not beside them | **graded, with the substitution now stated rather than glossed**: *ensouled* (the being's claim is read by `von`; an unreadable being is refused `no_reading`) and *laya/von-driven* (the choice is the stage question over the calibrated classifier, its floor declared in the charter before the answer exists) are met on existing seams with no second path. **Top gap, named and owed**: the third clause — *needs met through the game's own cellular automata* — is met by the tree's own cell-based water law **as one static evaluation at `Tick(0)`**, and **a real stepping cellular automaton on the field plane is not built**: state that carries tick to tick, so a rained-on or drained column changes the being's next step. This slice must not be read as one. |
| **Determinism and no partial state** — the plan's own bar | **graded and passed**: the binary's two runs are byte-identical, two runs of the first mob produce one chain digest (`979bf776`, unmoved), the being holds no mutable interior, and a failed act is recorded rather than half-taken. **Top gap**: determinism is measured at one tick of one world; nothing here reads one being across two seeds or two worlds, which is where a wandering digest would first show. |
| **Honesty and boundary** — what is claimed against what was run | **graded, with a named residue**: every number here is a printed value, a measured digest or an exit status; the nine `.test.log` bytes on disk are this pass's re-run's and §32's original nine are **unrecoverable** and recorded as such; §32's table now names the two files it had omitted; the abbreviated digests are expanded in the capture pair and in [`baselines/u12-repair/digests.txt`](baselines/u12-repair/digests.txt). **Top gap, owed**: the act's floor rests on **one declared fact that holds by absence** — a head that declares almost nothing, scored through a vocabulary that is not the being's own — and a being-act declared-fact vocabulary, so a floor can rest on a fact *declared present*, is owed. Nothing under `crates/` was read or written, and the four U10 rulings, `mc-ar-mod`, the `noul`/`score` sets, the score rungs and §5 rows 8, 11 and 13 stand untouched. |

### Owed, and named

**A stepping cellular automaton on the field plane** (the mission clause this slice substitutes for, named in
the run, the module header and the table above); **a being-act declared-fact vocabulary** (so the floor rests
on a fact about an act that is declared present, rather than on a patch record's dependency key by absence);
and, unchanged from §32: **U6's self-describing protocol**; **U2's T016 vocabulary** (the office rank and the
certification bands, printed owed in `axis.rs`); **the other kinds of being** (a plant, an incarnate pilot, a
god's office); **ensoulment's own slice** (soul-energy accounting, the released essence, consent/rite/
corruption); **relations and the court**; **apprenticeship/tutoring**; **a population** (many beings, one
world, and the picture of it); §24's remainder (ozone, an azimuth, the per-cell frame format); §29's
remainder; U1's remainder; the `noul`/`score` calibration sets; the **four U10 rulings**; `mc-ar-mod`; and §5
rows **8**, **11** and **13**. **This pass asked the user nothing**.

### The next buildable step

The reconnaissance for the next step is unchanged from §32's remainder and the mission's own order: **the
second being** — one world with two mobs, whose needs race for the same cells, which is where a *relation*
between beings becomes readable rather than asserted — or the **self-describing protocol** the row names, or
the **stepping automaton** on the field plane this pass named owed, which is the one item here that the
mission's own clause is waiting on.

*(2026-10-13, §34: the third of those three is **built** — see §34 below. The order this section named is
unchanged for what remains: the second being, then the self-describing protocol.)*

## 34. The field plane steps — a need met through the game's own automata over time (2026-10-13)

This pass built the item §33 named as **the one the mission's own clause was waiting on**: *a stepping cellular
automaton on the field plane* — state that carries tick to tick, so a rained-on or drained column changes the
being's next step (the mission's own clause for U12: *the game's cellular automata systems should allow for mobs
and autonomous players to meet their own needs through itself*). Reconnaissance first, and the record's own bar
for this row: §33's owed line, which is also the **top correctness gap** in §33's own four graded dimensions.
The record orders **no** different unblocked row ahead of it — §5 row 12 and §16.2 row 12 put U12 behind its two
owed prerequisites and name nothing before this item; the render/window line is U8's own remainder inside row 7,
and U13's closing row is put **last** by §16.2's own reasoning (*"completed first, it would be judged against
nothing"*), so the render line cannot be ahead of a row that feeds it; and the second being is inside row 12 with
nothing ordering it first. So the row was taken, and nothing else moved: the four U10 rulings, `mc-ar-mod`, the
`noul`/`score` sets, the score rungs and §5 rows 8, 11 and 13 stand as they stood, the library's only dependency
is still `sha2`, and **nothing under `crates/` was read or written** (P7).

### The law, and the readings it took rather than asking

1. **One cell state.** A water depth per column of the patch the read already names, in micrometres, starting
   from the world's own filled surface (`water_level − elevation`, U3's `hydrology` stage) — the state the
   generator left, not a plane of zeros, so the first tick routes a world rather than inventing one.
2. **One law.** The game's own routing: every column's own water leaves for the neighbour the shaper's own
   `d8_receivers` names (read from the read, never re-derived), and a **pit** — a column no neighbour is lower
   than, so no receiver is named — keeps a declared fraction of what it holds and loses the rest. One tick is
   one **synchronous** pass computed from the state at the *start* of the tick, so the answer cannot depend on
   the visit order — which is the property a cellular automaton is for.
3. **Two declared constants.** `RAIN_UM_PER_TICK = 200` and `PIT_KEEP_PERCENT = 90`, declared in one place,
   printed by the run and measured by the law's own tests. Rain alone would drain every column to zero and a pit
   that kept everything would never come to rest; the pair is what makes the plane a bounded world.
4. **One tick count.** The automaton starts at the read's own `Tick(0)` and counts from there; it has **no
   clock, no thread and no schedule**. Eight ticks of the world's law is the declared count
   (`evidence::STEPPING_TICKS`) — the same number the being's record declares as its reach.

### One concern, one owner

| file | what it owns, alone |
|---|---|
| `src/automaton.rs` *(new)* | the cell state, the law and the tick: `Automaton::{over, step, steps, tick, ticks, standing, deepest, read, digest}`, and `RAIN_UM_PER_TICK` / `PIT_KEEP_PERCENT`. No being, no need, no clock |
| `src/being.rs` | `WorldRead::stepped_from` — **the same world over another state**: patch, origin, terrain and receivers are the read's; only the water is the law's. The read's own `depth` doc says which state it stands. Nothing downstream re-derives water |
| `src/evidence.rs` | `check_u12_stepping` — the run's three lines and their bar — plus `STEPPING_TICKS` and `next_step`/`drank_at`, the one comparison of "its next step" that the acceptance instrument uses too |
| `src/main.rs` | the wiring, and the failure line's own list (`U12 stepping={…}`) |
| `src/systems.rs` | the `[U12]` registry row: the stepping clause added, the owed list beside it kept |
| `src/lib.rs` | the module list that publishes `automaton` — without it nothing here is reachable |
| `tests/u12_stepping.rs` *(new)* | the instrument: the state carries, the rained-on column changes the being's next step, two states from one read are one digest |
| `docs/baselines/u12-stepping/*` | this pass's captures, the eight-case guard proof, and the digests of every file it touched |

**Not built, and named rather than implied**: no second needs model, no second scheduler and no second automaton
path beside the tree's existing field plane; no overflow. A pit at rest holds the fixed point of its own
recurrence, `kept(holding + rain) + inflow`; the plane's one source is `cells × rain` per tick and its one sink is
the pits' thinning, so one pit taking the whole plane is bounded by `10 × cells × rain` = 4 608 000 µm (4.608 m),
and this patch's own sink comes to rest at 7 800 µm because its catchment is small. A pit that filled would keep
growing to that fixed point; **an outflow or an overflow is owed**.

### The measured movement

| measured | value |
|---|---|
| the plane at Tick(0) | 275 of 2 304 column(s) stand water; deepest 303 291 µm at column 662 |
| the plane at Tick(8) | 1 647 column(s) stand water; deepest 1 470 522 µm at column 424 |
| the dry sink (column 0) | 0 µm at Tick(0) → 4 147 µm at Tick(8) — **rained on** |
| the state digest | `0efc8cda` — two states from one read and one count are one digest |
| the being, cell 14, **Tick(0)** | 4 step(s), then drinks at column 159 (0.010996 m); chain `979bf776` (unmoved from §32) |
| the being, cell 14, **Tick(8)** | 1 step, then drinks at column 62 (0.000200 m — exactly one tick's rain); chain `9b0de07e` |
| the column it drinks at, Tick(8) | stood **no** water in the generator's filled surface: the water it drinks is the law's own |
| the pit's own rest (the law's test) | 7 800 µm, flat from tick 100 to tick 2 100, and the fixed point of the law's arithmetic at that inflow |

**Two planes, two chains.** The same being, at the same column of the same patch, walks four steps in the world
the generator left and one step in the world the law has moved — and drinks, in the second, at a column the
generator left dry. That is the whole difference between this slice and §32's static evaluation, and the run
prints it in one line.

### The capture before and after, and the movement accounted line by line

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), **504 lines** | `5f4d5c80…` | `f583ca18…` (1 line) |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **507 lines** | **`b0942531…`** | **`f583ca18…`** (1 line, unchanged) |

The pre pair is **§33's own final pair** — the same bytes, the digest §33 recorded — so this pass began exactly
where §33's account ended. The movement by `diff` is **two lines rewritten and three added, none removed**
(504 → 507, net +3), and each one is named:

* **rewritten, line 83** — the registry's `[U12]` row gains **The plane steps (2026-10-13, §34)** and its owed
  list gains the being-act declared-fact vocabulary;
* **rewritten, line 499** — `U12 first mob: … (one static evaluation of the roster at Tick(0), not a stepping
  cellular automaton — owed)` → `… (one static evaluation of the roster at Tick(0); the plane that evaluation
  left is stepped in the next line, §34)`;
* **added, lines 504–506** — the three `U12 stepped plane:` / `U12 stepping:` / `U12 stepping determinism:`
  lines.

Every other line is byte-identical, including §32–§33's own `U12` lines and the being's chain digest at the
static read (`979bf776`, still printed twice). Two direct runs of the final binary are byte-identical
(`b0942531…` twice), and the binary's own stderr did not move at all (`f583ca18…`, the one line §32 recorded).

### The guards, eight, proved to bite by name

| case | subject | what bit |
|---|---|---|
| the declared rain | `RAIN_UM_PER_TICK = 200` → `0` | the pit never wets (`rain_wets…`) |
| the fraction a pit keeps | `PIT_KEEP_PERCENT = 90` → `100` | the plane never comes to rest (79 400 µm and rising) |
| the one-hop transfer | `moved[to] += own;` → `+= own / 2;` | the per-column law, at column 0 |
| the pit's own thinning | the pit's `moved[cell] -= …` → `= 0` | the pit's own fixed point never arrives |
| the stepped read's own water | `stepped_from(&self.depth_um)` → a plane of zeros | "the read stands the law's water" fails |
| the tick count | `self.ticks += 1;` → `+= 2;` | the count is not the state's |
| the state the law starts from | `Automaton::over`'s initial depth → zeros | "the state the law starts from is the read's own water" fails |
| the being reads the law's state | the instrument's stepped drive → the unstepped plane | "the plane's own change must move the being's next step" fails, printing `(4, Some(159))` both times |

The harness ([`baselines/u12-stepping/run-guards.sh`](baselines/u12-stepping/run-guards.sh)) exited **0**: all
eight cases bit **by name**, each perturbation was first checked to match **exactly once**, and every perturbed
source came back **byte-exact** (`sha256sum -c` against the pre-run snapshot, restored through a `trap`):
`automaton.rs 1e4fa647…`, `being.rs 74b34020…`, `lib.rs 539bafbe…`, `evidence.rs 512b972d…`,
`u12_stepping.rs 10aaeff8…`. The eight `.test.log` files beside the proof are this run's; their digests are in
[`baselines/u12-stepping/digests.txt`](baselines/u12-stepping/digests.txt).

**One property is deliberately not a guard**: order-independence. The law is written as one change per column
computed from the start-of-tick snapshot, so reordering the visit cannot change the answer — there is no edit
that would make a *correct* build fail. What pins the arithmetic instead is the law's own test, which states
every column's new water as the arithmetic itself (what arrived, plus — for a pit — the declared share of what it
held) and then the conservation identity `after = before + cells × rain − drained`.

### The done conditions, with exit statuses

| Check | Exit | Measured on the final bytes |
|---|---|---|
| `cargo test --offline --all-targets` | **0** | **381 passed / 0 failed** across **14** suites (1 lib + 13 integration binaries) — **304** lib + 77 integration — §33's 374 plus this pass's four `automaton::tests::…` and three `u12_stepping` tests |
| `cargo clippy --offline --all-targets -- -D warnings` | **0** | `Finished dev profile` — no diagnostics, and no `allow` was added |
| `cargo run --offline` | **0** | 507 stdout lines (`b0942531…`), two runs byte-identical; the binary's own stderr is the one line `f583ca18…`, unmoved since §32 |
| the tree's own vocabulary | — | refusal names **92** (`refusal.rs:424`), event kinds **7**, namespaces **3**, actions **13** (the run's own `13 declared action(s)`), table verbs **8** — all **unmoved** |
| the guard harness | **0** | eight cases bit by name; five sources restored byte-exact (digests above) |

### The four graded dimensions, with the top gap in each

| Dimension | Verdict, and the top gap |
|---|---|
| **Correctness** — the record's own bar is met on the record's own bytes | **graded and passed**: the bar §33 recorded as owed is now a printed line — *a rained-on or drained column changes the being's next step* — measured as the same being, at the same cell, taking a different step (4 → 1) and drinking at a column the generator left dry. 381/0, all eight guards bit by name, and the law's arithmetic is pinned column by column with an exact conservation identity. **Top gap**: the plane has one sink and no outflow, so a pit grows to its fixed point and stays — the *world* is bounded but it never empties, and no test drives the plane past 2 200 ticks against a second world or a second seed. |
| **The named claim** — the mission's clause lands on existing seams | **graded and closed for this clause**: *needs met through the game's own cellular automata* is met **over time** now — the state carries, and the being's world read is that state (`WorldRead::stepped_from`), with the routing still the shaper's own `d8_receivers` and the water still U3's own filled surface. No second automaton, no second needs model and no clock were added. **Top gap**: the automaton steps *one* field (water) — the mission says *systems*, plural, and the other cell-based systems of the tree (`life`'s flocking, `weather`'s orographic gain, `ecology`) do not step; a being whose need is met by a *second* such system is owed. |
| **Determinism and no partial state** — the plan's own bar | **graded and passed**: two states from one read and one count are one digest (`0efc8cda`), the being over one stepped plane is one chain digest on two runs, the binary's two runs are byte-identical, and the state is a pure function of the read and the count — no clock, no thread, no interior mutability. **Top gap**: determinism is measured within one process on one platform: the *integer* law makes float formatting irrelevant, but nothing here pins a digest across architectures or across a save/load of the stepped state — which would need a save of the automaton's own state. |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: the pre pair is §33's own final bytes and the movement is named line by line (two rewritten, three added, none removed); every number in this section is a printed value, a measured digest or an exit status; the fixed-point claim was corrected *in this pass* to the law's own arithmetic after a first draft asserted an equilibrium the measurements refuted (the sink settles at 7 800 µm, not `rain / (1 − keep)`); the no-overflow limitation is named as owed rather than hidden. **Top gap, owed**: the being-act declared-fact vocabulary still stands — an act's floor rests on a declared fact that holds by *absence*. Nothing under `crates/` was read or written, and the four U10 rulings, `mc-ar-mod`, the `noul`/`score` sets, the score rungs and §5 rows 8, 11 and 13 stand untouched. |

### Owed, and named

**The stepping automaton is closed** (this pass). Owed now, in the order §16.2 puts them: **an outflow or an
overflow at a pit** (a world that can empty), **a second stepping system** (a need met by `life`, `weather` or
`ecology` rather than by water), **a save of the stepped state** (the automaton's own state in the record, so a
rebuilt world resumes at the tick it was at), **a being-act declared-fact vocabulary** (so a floor rests on a
fact about an act that is declared present); and, unchanged: **U6's self-describing protocol**; **U2's T016
vocabulary** (the office rank and the certification bands, printed owed in `axis.rs`); **the other kinds of
being** (a plant, an incarnate pilot, a god's office); **ensoulment's own slice**; **relations and the court**;
**apprenticeship/tutoring**; **a population** (many beings, one world, and the picture of it); **the second
being**; §24's remainder (ozone, an azimuth, the per-cell frame format); §29's remainder; U1's remainder; the
`noul`/`score` calibration sets; the **four U10 rulings**; `mc-ar-mod`; and §5 rows **8**, **11** and **13**.
**This pass asked the user nothing.**

### The next buildable step

**The second being** — one world with two mobs, whose needs race for the same cells — is the item §32's
remainder and §33's own close both put first, and it is now measurable in a moving world rather than a still
one: two beings whose walks diverge *because the plane between them changed* is where a **relation** between
beings becomes readable rather than asserted. The self-describing protocol the row names is the other candidate.

*(2026-10-13, §35: a read-only audit of this section named four defects — the automaton trusting a
caller-supplied read, a stale `water_level` beside the stepped `depth`, printed numbers that describe the
generator's lake in flight rather than the world at rest, and the audit's own measurement living only in a
transcript. All four are closed in §35, which also lands that measurement here.*)

## 35. The audit's defects closed — one owner for the stepped world, one water truth, and numbers that describe the world at rest (2026-10-13)

This pass closes **exactly** the four defect groups a read-only audit of §34 named, and adds no capability past
them: no second stepping system, no outflow, no save of the stepped state, and no restructuring of
`evidence.rs` (which is a later pass's business if it is anyone's). The library's only dependency is still `sha2`, no refusal name,
event kind, namespace, action or table verb moved (92 / 7 / 3 / 13 / 8, measured and printed), and **nothing
under `crates/` was read or written** (the newest mtime under `crates/` is still 2026-10-11 18:30, days before
this pass).

### The four defects, and where each is closed

| the audit's finding | what this pass did |
|---|---|
| **`Automaton` did not own the world it stepped.** `read(&self, base: &WorldRead)` trusted **any** caller-supplied read and returned `base.stepped_from(&self.depth_um)` — patch, terrain and receivers came from that read while the water came from the automaton, so a wrong base produced a silently mixed world (and a length mismatch read zeros rather than refusing) | the automaton **holds the world it steps**: `world: WorldRead` is a field set at `Automaton::over`, the routing graph is read from it (`self.world.downhill`), and the separately-kept `receivers` field is gone. **`read()` takes no base** — a mismatched world cannot be *expressed* rather than being silently mixed. `Automaton`'s derive loses `Eq` (the world carries floats) and `WorldRead` gains `Clone, Debug, PartialEq` |
| **Two water truths.** `stepped_from` copied the generator's `water_level` beside the law's `depth`, so every stepped read had `depth != water_level − elevation`; and because [`being::cells_digest`](clean-room/src/being.rs) hashes `elevation` and `water_level`, **a stepped world digested exactly as the static one** (measured before the fix — two different worlds, one digest) | `depth` is the **authority** and the surface is **derived** from it (`water_level = elevation + depth`), documented as such on the field and in `stepped_from`. `tests/u12_stepping.rs::the_reads_carry_one_water_truth` asserts the identity in **both** reads and asserts the two digests now differ. Dead `Automaton::cells()` deleted; the duplicated drink-scan removed — `drank_at` returns the drink **fact** (column and depth) and `next_step` and the run's formatter both read that one scan |
| **The printed numbers described a passing world.** §34's line printed tick 8, which the audit measured to be the generator's lake still in flight (deepest 1 470 522 µm, 12 886 183 µm on the plane) — not the world the being lives in | the run steps the plane **to rest** (`SETTLE_TICKS = 200`, declared in `src/automaton.rs`) and prints the tick-8 transient **beside** the settled figures; the being is driven over the **settled** plane, and its **need is printed as its own value** on both planes (`need reads met = …`) rather than being left to be read out of the act. New guard `automaton::tests::the_plane_comes_to_rest_and_the_settled_world_is_not_the_transient` pins the settled shape — the wet set equal at tick 8 and at rest, the total falling into the pits, the deepest below the transient peak, five hundred further ticks moving nothing, and the marsh claim (more than half the plane wet) |
| **The audit's measurement existed only in that conversation** — 71.5 % wet from tick 8, deepest 1 470 522 µm → 441 809 µm, total settling at 6 920 441 µm | landed here, with the numbers, in the table below — and the run now prints the settled figures itself, so the record and the product say the same thing |

### The measured numbers, now in the record

| measured | value |
|---|---|
| the plane at Tick(0) | 275 of 2 304 column(s) stand water; deepest 303 291 µm at column 662 |
| at **Tick(8)** — the transient §34 printed | **1 647** column(s) stand water; deepest **1 470 522 µm** at column 424; **12 886 183 µm** on the plane (the generator's lake still passing) |
| at Tick(50) | 1 647 column(s); deepest 448 985 µm at column 1271; 6 985 265 µm |
| at **rest — Tick(200)**, the state the being reads | **1 647** column(s) — the same set as at tick 8; deepest **441 809 µm** at column 1271; **6 920 441 µm**, and unchanged at Tick(1 000); state digest `19df3015` |
| the dry sink at rest | 7 800 µm — the fixed point the law's own unit test derives |
| the being at Tick(0) | need reads met; 4 step(s) then drinks at column 159 (0.010996 m); chain `979bf776` |
| the being at rest | need reads met; 1 step then drinks at column 62 (0.000200 m — one tick's rain, in a column the generator left dry); chain `9b0de07e` |
| `cells_digest`, stepped read | before this pass **equal** to the static read's (the stale surface); now **different** — the new test's own assertion |

### The capture before and after, and the movement accounted line by line

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), **507 lines** | `b0942531…` | `f583ca18…` (1 line) |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **507 lines** | **`d14588a5…`** | **`f583ca18…`** (1 line, unchanged) |

**Where the pre-capture came from**, plainly: it is **§34's own final pair**, copied in — the tree this pass
began from, and the tree the audit graded. The pre-pass binary is not rebuildable (the audit produced no
capture of its own), so this pass's pre pair is inherited **by digest** rather than re-taken: `b0942531…` is
the value §34 recorded for those bytes, and `unmodified` is what `cmp` says about it against §34's file. Every
baseline directory in this record that reuses a pair says so in the same words.

The movement by `diff` is **three lines rewritten, none added, none removed** (507 → 507, one hunk
`-504,3 +504,3`), and each is named:

* **line 504** — `U12 stepped plane: 8 tick(s) …` → `U12 settled plane: 200 tick(s) …`, which now prints
  Tick(0), the Tick(8) transient (*"the generator's lake still passing"*), the settled figures, the total
  *"unmoved by 200 → 700 tick(s)"*, the sink at rest, and the settled state digest `19df3015`;
* **line 505** — `U12 stepping: … its next step is 4 step(s) then drink (column 159) and it drinks at column
  159 …` → `… its need reads met = true (at column 159, 0.010996 m …, next step 4 step(s) then drink (column
  159)); over the settled plane at rest its need reads met = true (…)`, so the **need** is a printed value on
  both planes;
* **line 506** — `U12 stepping determinism: two states from the same read and 8 tick(s) …` → `… 200 tick(s)
  … (19df3015) … twice over the settled plane is one digest = true (9b0de07e); the two planes give two chains
  = true (979bf776 at Tick(0), 9b0de07e at rest)`.

Every other line, digest and count is byte-identical, including §32–§34's own `U12` lines and the being's
static chain (`979bf776`, printed three times). Two direct runs of the final binary are byte-identical
(`d14588a5…` twice), and the binary's own stderr did not move (`f583ca18…`, the one line §32 recorded).

### The guards, ten, proved to bite by name

Eight cases carried over with their subjects (the declared rain, the fraction a pit keeps, the one-hop
transfer, the pit's own thinning, the stepped read's own water — whose subject moved to `self.world.stepped_
from(&self.depth_um)` now that `read()` takes no base — the tick count, the state the law starts from, the
being's own read of it), and two are new for this pass's guards:

| case | subject | what bit |
|---|---|---|
| the settle count | `SETTLE_TICKS = 200` → `1` | the settled-shape test: a state one tick in is not at rest |
| the water truth | the surface derivation `.map(|(ground, water)| ground + water)` → `*ground` | `the_reads_carry_one_water_truth`, at its identity |

**One case was corrected rather than kept**, and it is worth recording: the water-truth subject was first
written as the `water_level,` field initializer with the generator's surface substituted for it, and it bit
with `error[E0283]: type annotations needed` — a **compiler failure**, not the guard. A compiler failure is
not a guard biting (it proves the build is brittle, not that the test detects the defect), so the subject was
moved to the derivation itself, which compiles and fails the assertion. The harness exited **0** with all ten
cases biting by assertion or panic, each perturbation checked to match **exactly once**, and every perturbed
source back **byte-exact** (`sha256sum -c` through a `trap`): `automaton.rs 37ebc2a3…`, `being.rs f5351ae6…`,
`lib.rs 539bafbe…` (untouched this pass), `evidence.rs b5288b40…`, `u12_stepping.rs 5c4fbdd9…`.

### The done conditions, with exit statuses

| Check | Exit | Measured on the final bytes |
|---|---|---|
| `cargo test --offline` | **0** | **383 passed / 0 failed** across 14 suites (1 lib + 13 integration binaries) — **305** lib + 78 integration — §34's 381 plus this pass's settled-shape unit test and water-truth instrument |
| `cargo clippy --offline --all-targets -- -D warnings` | **0** | `Finished dev profile` — no diagnostics, and no `allow` was added |
| `cargo run --offline` | **0** | 507 stdout lines (`d14588a5…`), two runs byte-identical; the binary's own stderr is the one line `f583ca18…`, unmoved since §32 |
| the tree's own vocabulary | — | refusal names **92**, event kinds **7**, namespaces **3**, actions **13**, table verbs **8** — all **unmoved** |
| the guard harness | **0** | ten cases bit by name; five sources restored byte-exact (digests above) |

### The four graded dimensions, with the top gap in each

| Dimension | Verdict, and the top gap |
|---|---|
| **Correctness** — the record's own rules refuse, and the numbers describe the world | **graded and passed**: the four defects are closed and each is pinned by a guard that was shown to bite; the settled state is the state the being is driven over, and the digest the run prints (`19df3015`) is the digest an independent out-of-tree probe reached at tick 200 — two routes to one number. 383/0, ten guards bit by name, exit 0 on all three checks. **Top gap**: the law still has no outflow, so the settled world is a **marsh** — 71.5 % of columns permanently wet and a need met by a 200 µm film; nothing measures whether that is the world this row wants. That is a product ruling, not a defect, and it is owed below. |
| **Design** — one owner per piece of state, a change landing in one obvious place | **graded and passed for what this pass owns**: the automaton owns the world it steps (and the routing graph is not kept a second time), `read()` has no argument to get wrong, the water has one authority (`depth`) with its surface derived, and the drink scan has one owner. **Top gap**: `Automaton::over` now **clones the whole read** (four vectors) and `read()` clones three of them back — for a 2 304-column patch that is immaterial, but the ownership fix bought correctness with copies, and a borrowed form (a lifetime, or a `Cow`) is the cleaner answer at planet scale. `src/evidence.rs` (4 198 lines — **corrected 2026-10-13**: this line printed 4 121; §36's own last bytes measure **4 198**, and §37 leaves **4 378**) was **deliberately not restructured** this pass. |
| **Determinism and no partial state** | **graded and passed**: two states from one read and one count are one digest (`19df3015` at rest), the being over one plane is one chain digest on two runs, the binary's two runs are byte-identical, and the state is a pure function of the read and the count. **Top gap**: pinned within one process and one platform, and the automaton's own state is still not saved — a rebuilt world resumes at `Tick(0)` and would replay the plane from the generator's surface, not from where it was. |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed**: the audit's measurements are in §35's table rather than in a transcript; the pre-capture's provenance is stated in the words above; the guard that *failed to bite honestly* was corrected and the correction recorded rather than hidden; every figure here is a printed value, a measured digest or an exit status; the marsh limitation is named as owed rather than left as a headline. **Top gap, owed**: the being-act declared-fact vocabulary still stands — an act's floor rests on a declared fact that holds by *absence*. Nothing under `crates/` was read or written, and the four U10 rulings, `mc-ar-mod`, the `noul`/`score` sets, the score rungs and §5 rows 8, 11 and 13 stand untouched. |

### Owed, and named

**Closed this pass**: the audit's four defects. **Owed now, in §16.2's order**: **the marsh ruling** (does a
world where 71.5 % of columns are permanently wet and 200 µm of water meets a need be the world this row
wants — an outflow, an infiltration, a depth floor for drinking, or none of them); **a second stepping
system** (a need met by `life`, `weather` or `ecology` rather than by water); **a save of the stepped state**
(the automaton's own state in the record, so a rebuilt world resumes where it was); **a borrowed rather than
cloned world in the automaton**; **a being-act declared-fact vocabulary**; and, unchanged: **U6's
self-describing protocol**; **U2's T016 vocabulary**; **the other kinds of being**; **ensoulment's own
slice**; **relations and the court**; **apprenticeship/tutoring**; **a population**; **the second being**;
§24's remainder; §29's remainder; U1's remainder; the `noul`/`score` calibration sets; the **four U10
rulings**; `mc-ar-mod`; and §5 rows **8**, **11** and **13**. **This pass asked the user nothing.**

### The next buildable step

Two candidates, and the record's own order still puts the first first: **the second being** — one world with
two mobs whose needs race for the same cells, now measurable in a settled world rather than a passing one — and
the **marsh ruling**, which the numbers above make decidable by the user in a sentence and which everything
else about a being's need now depends on.

## 36. U1's visible client — the world's own cells drawn, presented in a real window, and the frame's pixels read back (2026-10-13)

This pass takes **§5 row 1's own remainder** — *a **visible** client (never `nodraw`)*, whose check is *the band
count and a screenshot* (`LEDGER.md` **C-40**) — and builds it: `src/surface.rs` owns the client's pixels;
`Surface::render` draws the world the tree already builds through the contract's **one** `presentation::shade`
call; `present_world` opens a **real** window against the platform's own interface, advances the plane's own law
**one tick per frame**, hands the frame to the screen, and then reads the window's own client area back and
compares it, byte for byte, with the surface it owns. Nothing past that is claimed: there is **no GPU pass**
(a shader beside `shade` must be *checked against* it, and a second render path that is not compared is a second
truth), **no resizable window**, **no camera or scene graph**, and **no azimuth** for the Sun (§24's own owed
item, printed as the plane it is placed in rather than invented). The library's only dependency is still `sha2`
(`Cargo.toml` declares it and its own comment says why), and the tree's vocabulary moved in exactly one place —
**refusal names 92 → 94** (this pass's `no_reflectance` and `platform_refused`), with event kinds **7**,
namespaces **3**, actions **13** and table verbs **8** unmoved. **Nothing under `crates/` was read or written**:
the newest mtime there is still **2026-10-11 18:30** (`crates/voxel-appearance/src/sky.rs`), two days before this
pass.

### The rulings this pass **took** rather than asked

| # | The ruling | Why it is the one the record requires |
|---|---|---|
| 1 | **No crate is added.** The window is fresh code against the platform's own interface (`surface::platform`, Win32), not a windowing dependency | the dependency rule is stated three times — the manifest (*"a dependency must name, beside its line, what it does that we need and that it is actually used"*), the plan's §2, and `.github/instructions/rust.instructions.md` as **Zero External Dependencies** (*"Dependency changes violate the zero-external-dependencies constraint"*) — so a windowing crate would be a **policy breach**, and the ruling decided *how* rather than *whether* |
| 2 | **`unsafe` is licensed in exactly one module.** `src/lib.rs`'s `#![forbid(unsafe_code)]` became `#![deny(unsafe_code)]`, and `surface::platform` alone carries `#[allow(unsafe_code)]` (`src/surface.rs:629`) | a window *is* a platform call and a platform call is `unsafe`; `forbid` cannot be narrowed, and a rule that cannot be narrowed would force a second crate for one door. Every other module is under the same denial as before, and `grep -rn "allow(unsafe_code)" clean-room/src` returns that **one** code line (the four other matches are the prose that records the decision) |
| 3 | **A frame is a tick.** The window's loop advances the plane's own law **once per frame**, and the declared count **is** the law's own horizon: `DECLARED_FRAMES = SETTLE_TICKS = 200` | so the picture a person watches *is* the world the run's other lines measure — the window's last frame and the evidence run's frame are the same frame, which is pinned by a test rather than promised. There is **no second scheduler**: the loop is the only clock |
| 4 | **The Sun's tint is the solver's own chromaticity *shape*, normalised to its brightest channel.** `light::Solver::spectra().response(&solver.sun_transmittance())` returns a chromaticity (~0.09 in magnitude), so its own absolute scale was dimming the frame to a near-black picture | `light.rs` says the same thing itself — *"a 5 778 K body renders near the model's reference white"* — so the client keeps **which** colour the air column makes the Sun (it reddens as the Sun lowers) and drops the table's absolute scale, which is the table's business and not the frame's. The tint the light carries is **printed** by the run (`(1.000, 0.925, 0.733)` at 35°), so the decision is a measured number rather than a hidden constant |
| 5 | **The encoding is BGRA and the read-back is a top-down `BI_RGB` 32-bit DIB**; the screenshot is **P6 RGB** — the same bytes, reordered for a file | the platform's own buffer order is a fact about the platform, so it lives at one site (`encode`) and one test pins it (`red` belongs in the *third* byte). The read-back DIB is top-down so the comparison is against the surface's own byte order, not a flipped one; the P6 file is RGB because that is what a P6 file is, and the file's first pixel is asserted to be the surface's first pixel **in RGB order** — the screenshot is the bytes the window was handed, not a second drawing |

### One concern, one owner

| concern | owner |
|---|---|
| the **pixels** the client holds | `Surface` — one BGRA buffer, its own size, its own digest, and the shaded samples behind it |
| the **drawing** — world to pixels | `Surface::render` — one loop over the world's own columns, one `shade` call each, materials from the ladder, all through the one seam |
| the **presentation** — pixels to a window | `platform::Window` — one class, one window, one `StretchDIBits` per frame, and the read-back that says the screen holds what the surface owns |
| the **one call site** that drives it | `present_world` — the world the tree already builds, one tick per frame, and the measured report |
| the **screenshot** | `Surface::write_ppm` — the presented frame's own bytes as a P6 file, so the eye's check has an artifact rather than a promise |
| the **light and the view** | `surface::light` (U5's solver, in the declared state) and `surface::view` — the client adds nothing but the declared altitudes |
| the **normal of a column** | `surface::normal_of` — level where the world's own law stands water, the terrain's own gradient elsewhere, with `RELIEF_EXAGGERATION` the only thing this client adds |

### The measured numbers

| measured | value |
|---|---|
| the frame | **576 × 576** pixel(s) from **2 304** shaded column(s) — the patch's own side, at **12** pixel(s) per column |
| the frame's own contrast | **39** distinct luminance band(s) on the frame's own 2 304 sample(s) (≥ **3** required, resolution 64); channels **0x47..0xfb** |
| the frame's own digest | `6e1d9c96…` (the pixels), over the world `cells_digest` `88304a75…` and the plane's own state digest `19df3015…` |
| the world it drew | the plane's own law at rest (200 tick(s)): **1 647 of 2 304** column(s) stand water, deepest **441 809 µm** at column 1271 — §35's settled figures, read here rather than re-derived |
| the light | U5's solver's own Sun at **35°**, tint `(1.000, 0.925, 0.733)`, view at **15°** |
| the materials | classes 0→1 1→1 2→2 3→3 4→4 5→6, with water = material **5** — the reference world's own ladder, printed and asserted against its own mapping |
| the window | **576 × 576** client pixel(s), dpi-aware = **true**, **200 of 200** frame(s) presented at **30** frame(s)/s |
| the read-back | the window's own client area, through the declared 32-bit top-down `BI_RGB` DIB, is the surface's own bytes at **66 355 200 of 66 355 200** pixel(s) — every pixel of the client area, every one of the 200 frames |
| the screenshot | `frame.ppm`, **995 343** bytes: `P6\n576 576\n255\n` + the frame's own bytes; digest `86daeaf3…`; **two windowed runs write the same file byte for byte**, and its first pixel `[0x47, 0x58, 0x64]` is the surface's first pixel in RGB order (the surface's own BGRA `[100, 88, 71]`, i.e. red `0x47` = 71, the same first byte the run prints as its dark end) |

### The capture before and after, and the movement accounted line by line

| Capture | Stdout | Stderr |
|---|---|---|
| pre (`pre-stdout.txt` / `pre-stderr.txt`), **507 lines** | `d14588a5…` | `f583ca18…` (1 line) |
| **final** (`final-stdout.txt` / `final-stderr.txt`), **510 lines** | **`3bd01135…`** (a **re-take**, after this pass's own report landed in `patches/` — the reason is line 490 → 493 below) | **`f583ca18…`** (1 line, unchanged) |
| the windowed run (`window-stdout.txt` / `window-stderr.txt`) | `59163d9c…` (5 lines) | *(empty — nothing in a windowed run writes to stderr)* |

**Where the pre-capture came from**, plainly: it is **§35's own final pair**, and `cmp` against §35's file says
`unmodified`. §35's bytes are recoverable — they are that pass's own `final-stdout.txt` — so this pass's pre pair
is inherited **by digest** rather than re-taken: `d14588a5…` is the value §35 recorded for those bytes (`cmp`
against `docs/baselines/u12-settled/final-stdout.txt` agrees). Every baseline directory in this record that
reuses a pair says so in the same words.

The movement by `diff` is **three lines added, three rewritten, none removed** (507 → 510), and each is named:

* **line 98–100 — added**, the client's own three `U1 client:` lines: the world it drew (1 647 of 2 304 columns
  at the plane's own rest, through the contract's own one `shade` call), the frame's own figures (39 bands, three
  digests, the channel range) and the fact that `window` is the eye's own check (C-40) — the last of these is the
  line that tells a reader the *screenshot* half of row 1's check exists and how to reach it;
* **line 6 — rewritten**, the `[U1]` registry row: `specified + one measured behaviour` → the same plus *"the
  visible client built (2026-10-13, §36)"*, what `src/surface.rs` owns, and what it still owes **by name** (the
  GPU pass, a resizable window, the Sun's azimuth);
* **line 11 — rewritten**, the row's `interface:` line, which gained
  `surface::{Surface::render → pixels, present_world → a real window, measured_frame → the frame's own figures}`
  and the binary's `window` mode.
* **line 490 → 493 — rewritten**, and **this pass's own report moved it**: U7's vocabulary instrument reads
  **every `patches/*.md` record in the tree, in place** (`src/von.rs::claimed_vocabulary_at`), so adding a record
  moves this line. The movement is exactly what the added record carries — records **39 → 40**, `user-directed`
  **23(+23 outside) → 24(+24 outside)**, and *"24 → 25 of the readings are records the calibration was **not**
  taken over"* — with the line's own digest (`d803c643`) and *"every word's claim held = true"* unmoved. That
  count is the instrument's own point: the word's coverage grew by a record the classifier was **not** calibrated
  on, which is what makes the reading measured rather than fitted. The final pair above is a **re-take** taken
  after `PATCH-040` landed for exactly this reason; the pre pair is still §35's.

Every other line, digest and count is byte-identical, including §32–§35's own `U12` lines, the being's static
chain (`979bf776`) and the settled chain (`9b0de07e`). Two direct runs of the final binary are byte-identical
(`3bd01135…` twice); its stderr did not move (`f583ca18…`, the one `serve:` line §32 recorded); and the windowed
pair's five lines are byte-identical across two runs **given the same frame-count argument** — the one argument
that does move the report is the screenshot's own path, which the report names.

### The guards, twelve, proved to bite by name

Nine are this pass's own new subjects, and three pin what the pass had already written:

| case | subject | what bit |
|---|---|---|
| the world's own water read | `let wet = world.depth_um(cell) > 0;` → `false` | the material test: a wet column drawn as its terrain's material |
| the level water surface | the wet column's normal `[0,0,1]` → `unit([0,1,0])` | the material test, at *"water drawn as a level surface"* |
| the declared relief | `RELIEF_EXAGGERATION = 8.0` → `0.0` | the material test: no exaggeration leaves no slope in the frame |
| the refusal's own name (`no_reflectance`) | `RefusalKind::NoReflectance` → `ReflectanceOutOfRange` | `a_material_that_derives_no_colour_refuses_the_frame_by_name`, on the kind |
| the zoom guard | `if zoom == 0` → `if false` | the zoom/column-count test, at the zoom |
| the column-count guard | `if classes.len() != cells` → `if false` | the same test, at the count |
| the declared frame count | `DECLARED_FRAMES = SETTLE_TICKS as u32` → `1` | `the_declared_frame_count_is_the_horizon_the_run_settles_at` |
| the encoding's own byte order | `[b, g, r, 255]` → `[r, g, b, 255]` | the lib test: *"red encodes to the third byte, because the buffer is BGRA"* |
| the screenshot's own channel order | the P6 writer's `[pixel[2], pixel[1], pixel[0]]` → `[pixel[0], …]` | the screenshot test, on the file's first pixel |
| the client area is the frame's | `client.right - client.left != width` → `!= width + 1` | the window test: *"the platform made a 576 × 576 pixel(s) client area for a 576 × 576 …"* |
| the blit's own answer | `if lines != self.height` → `!= self.height + 1` | the window test: *"the platform's own blit copied 576 of 576 scan line(s)"* |
| the screen holds the frame | the read-back's own comparison → `if false && …` | the window test: *"…the surface's own bytes are [100, 88, 71, 255] and the window's are [100, 88, 71, 0]"* |

The harness exited **0** with all twelve biting by assertion or panic, each perturbation checked to match
**exactly once**, and every touched source back **byte-exact** (`sha256sum -c`, through a `trap`):
`surface.rs 70b9744b…`, `main.rs e134c77e…`, `lib.rs 1b0ab8a2…`, `refusal.rs 098d6bb4…`,
`reference.rs 968c6b77…`, `evidence.rs 16df8fe4…`, `systems.rs c3b5e241…`, `tests/u1_surface.rs ddc54a9e…`,
`tests/u5_measure.rs fac97225…`. The case logs are deliberately **not** kept in the tree: they carry each run's
test-thread PIDs and cargo timings, so a re-run rewrites them — §33 recorded that trap against §32's nine logs,
and this harness keeps its logs in the scratch directory it makes.

### The near-miss this pass found, and the check it added

**A guard case was nearly read as a rendering defect, and the real cause was the harness itself.** After the
twelve cases above had run, a windowed run reported the read-back disagreeing at **0 of 66 355 200** pixel(s) —
and, at (0, 0), the two buffers held **equal** bytes (`[100, 88, 71, 255]` against `[100, 88, 71, 0]`). Since a
shorter run had agreed at every pixel minutes earlier, the natural reading was a frame-count threshold: some
longer runs "disagreed". The threshold does not exist. **`cargo test` builds the *binary* as well as the test
harnesses**, so the case that perturbs the read-back's own comparison (`if false && …`) left
`target/debug/shploingus-clean-room` as a **perturbed build** — and every later `window` run then used that
binary, whose comparison is short-circuited to `false`. That is why the mismatching pixel had equal bytes: the
comparison had been *forced* false, not defeated by the screen. A rebuild from the restored sources agreed on
every pixel again, and the `window N` counts that were being compared against each other were never measuring
what they were assumed to measure.

The harness now closes that hole rather than documenting it: **every case rebuilds** from the restored sources,
and the run ends by executing the rebuilt binary and comparing its own **stdout and stderr** against this
baseline's frozen capture — a tree that came back byte-exact and a build that did not would pass every other
check in the file and still be the wrong program. That last check is also the pass's own end-to-end statement:
the harness leaves behind a build whose output *is* `final-stdout.txt` / `final-stderr.txt`, byte for byte.

### The finding the bar itself made: this pass's own report failed U7 first, and the head was corrected rather than the instrument

`PATCH-040` was first written with `status: proposed`, and **the run stopped exiting 0**: U7's row read `false`,
stdout lost its closing `Clean room OK:` line (509 lines instead of 510), and the binary named the fault on
stderr — `Clean room FAILED: a measured bar is not met (… U7=false …)`. The cause is the record format's own law,
in its own words: `von.rs`'s claim for the word `user-directed` is ***"a user direction is applied"***, and its
reading refuses **by name** any record whose head carries that word at another stage (here `proposed`) —
*"a record that contradicts it is refused by name rather than counted as agreement"*.

The repair was to the **head** and not to the instrument. The applied change *is* the report, so the record now
declares `operation: report` and `status: applied`, and says in its own body that the **audit** it asks for is
requested rather than performed. No assertion was relaxed, no record was moved out of the corpus, and the
correction stays **measured**: the line above records it as 39 → 40. A reader can re-run the failing state in one
edit — put `status: proposed` back and the bar falls again — which is the whole point of a bar that reports
`false` rather than a run that quietly printed a smaller number.

### The done conditions, with exit statuses

| Check | Exit | Measured on the final bytes |
|---|---|---|
| `cargo test --offline` | **0** | **393 passed / 0 failed** across 14 suites (1 lib + 13 integration binaries) — **307** lib + 86 integration — §35's 383 plus this pass's two unit tests (`surface::tests`, the encoding and the one normalisation) and its eight integration tests (`tests/u1_surface.rs`) |
| `cargo clippy --offline --all-targets -- -D warnings` | **0** | `Finished dev profile` — no diagnostics, and no `allow` was added |
| `cargo run --offline` | **0** | 510 stdout lines (`3bd01135…`, the re-take), two runs byte-identical; the binary's own stderr is the one line `f583ca18…`, unmoved since §32 (except the one line §36's own report moved, above) |
| `cargo run --offline -- window 200 <frame.ppm>` | **0** | 5 stdout lines (`59163d9c…`), nothing on stderr, 200 of 200 frames at 30 f/s, the read-back agreeing at **66 355 200 of 66 355 200** pixel(s), and a P6 file (`86daeaf3…`) two runs write identically |
| the tree's own vocabulary | — | refusal names **94** (`92 → 94`: `no_reflectance`, `platform_refused`), event kinds **7**, namespaces **3**, actions **13**, table verbs **8** |
| the guard harness | **0** | twelve cases bit by name; nine sources restored byte-exact; the rebuilt binary's own output is the frozen capture |

### The four graded dimensions, with the top gap in each

| Dimension | Verdict, and the top gap |
|---|---|
| **Correctness** — the record's own rules refuse, and the numbers describe the world | **graded and passed**: the frame is drawn from the world the tree already builds, through the contract's **one** `shade` call — the test recomputes a column's sample from the material record rather than comparing it with a constant — and the picture reaches the screen: the window's own client area is the surface's own bytes at **66 355 200 of 66 355 200** pixel(s), with each of the guards that stands behind that sentence shown to bite. The band count is the contract's own instrument read on this frame's samples (39 ≥ 3), the screenshot is a real file two runs write identically, and the frame, world and state digests are printed. 393 / 0, twelve guards bit by name, exit 0 on all four checks. **Top gap**: the read-back is pinned **within one process, one platform and one pixel format** — 32-bit top-down `BI_RGB` at the declared dpi, on this machine's compositor. Nothing measures what a *different* desktop scale, another DIB format, or a compositor that hands the window a copy would do, and the loop's own panic-free behaviour under a mid-frame window close is exercised only by hand. |
| **Design** — one owner per piece of state, a change landing in one obvious place | **graded and passed for what this pass owns**: one module owns the pixels, one function draws them, one function presents them, one call site drives the loop, one site encodes a colour and one function writes the screenshot; the light is the solver's (not a second table), the ladder is the reference world's (so a class cannot draw one material and mesh another), and the plane's law is the automaton's — the client adds one constant (`RELIEF_EXAGGERATION`) and one view direction, both declared at the top of the file and both printed by the run. Each ruling above is a decision *taken* at one site rather than a default. **Top gap**: the platform is reached **through one module but not through one abstraction** — `platform` is Win32 by name (`StretchDIBits`, `BITMAPINFO`, a class registration), so the client is a Windows client rather than a client with a platform seam, and the honest next step is an interface the window is built against so a second platform is a second file and not a second module's worth of `unsafe`. `src/evidence.rs` (4 198 lines — **corrected 2026-10-13**: this line printed 4 121; §36's own last bytes measure **4 198**, and §37 leaves **4 378**) was again deliberately not restructured. |
| **Determinism and no partial state** | **graded and passed**: two renders of one world are one surface and one digest; the windowed report's frame digest is the evidence run's frame digest (the declared count *is* the settle horizon, asserted); the plane's law moves the picture (one tick changes the frame, and the settled plane is not the plane the being was placed in); two runs of the binary are byte-identical; the P6 screenshot is byte-identical across runs; and the run's own report is identical for the same argument — the only argument that moves the output is the screenshot path, which the report names. **Top gap**: pinned within one process and one platform, and the client's frame is a pure function of the world **at `Tick(0)` plus a count**: the plane's state is still not saved (§35's own gap), so a re-launched window replays from the generator's surface rather than resuming. |
| **Honesty and boundary** — what is claimed against what was run | **graded and passed, and this pass is the strongest case of it in the record**: the near-miss above is written down as a **harness** defect with the mechanism (a perturbed *build*, not a perturbed tree), including the equal-bytes evidence that explains it, rather than as a rendering bug that went away; the pre-capture's provenance is stated in the words §35's is; the screenshot is a shipped artifact with its own digest rather than a promise; every figure here is a printed value, a measured digest or an exit status; and the client's owed work is named on the row it belongs to rather than left as an implication. **Top gap, owed**: the frame is drawn **on the CPU**, so the `shade`/shader agreement the spec's own US2 asks for is still unmeasured — a shader beside this contract is a second truth until it is checked against the same seam, and the *look* (which palette, how the marsh reads to an eye) remains the user's, as `docs/APPEARANCE.md`'s header says. Nothing under `crates/` was read or written, and the four U10 rulings, `mc-ar-mod`, the `noul`/`score` sets, the score rungs and §5 rows 8, 11 and 13 stand untouched. |

### Owed, and named

**Closed this pass**: §5 row 1's own remainder — the visible client, with the band count *and* a screenshot, the
read-back that says the screen holds the frame, and the guards behind each of those sentences. **Still owed on
row 1**: the **GPU pass** (a shader checked against `presentation::shade` on the same surface — T009's other
half, and the only part of row 1's own check left), a **resizable window**, an **azimuth** for the Sun (§24's own
owed item, deliberately not invented), the **reduced OpenPBR parameter set's remainder** (T008, still open) and
the sky's **radiance as a term** (this client reads the solver's Sun — its direction and its own transmitted
colour — but not the sky's radiance, so nothing here yet lights a surface *by the sky*).

**And one thing this pass cannot leave as a gap, because the user has named it since: the shape.** Every Owed
list in §35 and §36 describes **a column world** — a square patch of cells with one class and one water depth
each — while the record's own coordinates are **cubic** (`src/coordinates.rs`: `32³` chunks on three axes,
`docs/DESIGN-SPEC.md:41`) and the witness is a camera into voxels. `PATCH-040` is the report of that finding and
the **audit** it asks the mission planner to run before any further slice; nothing else in this section should be
started ahead of that ruling.

**And the user ruled it in the same session, which closes the question rather than opening it** — their words:
*"the cubic chunks format caused massive rendering bugs which is why the clean room branch exists- it is the reason why
there was materials flickering in wgsl. This is why we can't use it"*. So the shape is not a choice between two
candidates: the **column representation is refused for cause**, and the cause is a **rendering** defect in the
**appearance** path — U1's own — which means the client this pass built is a **correct client of the shape the
product refuses**. What the audit still owes is therefore (a) the **mechanism** — where a per-column material
read reaches a shader, as a first-class finding rather than a symptom — and (b) how much of the column world
survives as a ***reading of*** a cubic world rather than as a world of its own.

**Owed now, in §16.2's order**: the **shape's consequences** (`PATCH-040` — the format is ruled **cubic** and the
column representation is **refused for cause**, so what is owed is the flicker's **mechanism** and the migration
order); the **marsh ruling**
(§35's item, unchanged — the numbers are printed and the ruling is the user's); **the second being**; **a save of
the stepped state**; **a borrowed rather than cloned world in the automaton**; **a being-act declared-fact
vocabulary**; **U6's self-describing protocol**; **U2's T016 vocabulary**; **the other kinds of being**;
**ensoulment's own slice**; **relations and the court**; **apprenticeship/tutoring**; **a population**; §24's
remainder; §29's remainder; the `noul`/`score` calibration sets; the **four U10 rulings**; `mc-ar-mod`; and §5
rows **8**, **11** and **13**. **This pass itself asked the user nothing** — every ruling above is recorded with
the rule it follows, and the one open *product* question it inherited (the marsh) was already the user's before
this pass began.

### The next buildable step

**Not the GPU pass, and not row 13's `P8` gate** — both are the next **column**-shaped steps, and the column
representation is now **refused for cause**: the user's own report is that it produced the material flicker in
WGSL, which is why this branch exists. A shader checked against `presentation::shade` would therefore be a shader
for a picture the product refuses — so the GPU pass is owed as a shader over **voxel faces**, against the
mechanism `PATCH-040` asks the audit to establish. The order the patch names: **the mission planner runs the
audit** (the shape is ruled; the flicker's mechanism and the migration order are what remain), and the GPU pass,
row 13's gate and the marsh ruling follow it rather than race it.

## 37. The shape ruling and the cubic world's first slice — four rulings recorded, the cells and their one identity, and the reading an existing system consumes (2026-10-13)

`patches/PATCH-040-the-column-world-against-the-cubic-chunk-format.md` named **four rulings** this record
owed and left them unanswered: **the shape**, **the identity**, **the seam** and **the client**. The user
ruled the cubic chunks format **refused for cause** — the material flicker in the WGSL path, reported by the user,
and that report is why this branch exists — so this pass **took the four rulings from the tree's own lines**
rather than asking again, recorded each as ruled in `docs/LEDGER.md` **C-189**, and built the smallest
complete first slice of the cubic world that the rulings require. §36's own "next buildable step" (the GPU
pass held back because a shader would be a shader for a picture the product refuses) is **superseded** by
ruling 4 and this section says so rather than leaving two records disagreeing.

One caveat, said plainly because the record owes it: **the flicker's mechanism is not established here.** This
tree contains no WGSL and no shader, so nothing in it can reproduce the defect; what this pass carries is the
user's report as the **cause** of the ruling, and what it can check — the client's own pixels — it checked:
**the shape ruling moved no pixel** (below).

### The four rulings, and the evidence each one carries

1. **The shape: the product is the cubic chunk, and the column world survives as a *reading*.** The world is
   the declared `32³` chunk (`docs/DESIGN-SPEC.md` §1.2, §5) on `coordinates`'s own algebra; `src/chunk.rs`
   owns the cells (`BTreeMap<ChunkKey, Chunk>`, air **named** rather than defaulted, water in whole
   micrometres) and `CubicWorld::column` is the one reading — the top solid cell, the water above it, its
   material, and how many solid cells stand under it. `being::WorldRead::over_cubic` hands the **same**
   `WorldRead` out of the cells, so the automaton, the being and the client consume it unchanged: one world
   model, no second routing law. Evidence: **4** chunk(s) × 32 768 = **131 072** cell(s) hashed, **24 689**
   solid; at column 1271 → (23, 26) the generator's Tick(0) reads surface **73.5539 m** and **127 536 µm** of
   water, the cells return **73.0000 m** (Δ **0.5539 m** < 1 m) and the same **127 536 µm** over **10** solid
   cell(s); the water round-trips exactly at **2 304 of 2 304** columns.
2. **The identity: the world's identity is the cells'.** `CubicWorld::digest` — every chunk key and every
   cell, byte-framed, in canonical order, with the patch and the vertical span framed in — is the world's one
   identity, `9d2eea8c…`. `being::cells_digest`, which hashes two per-column arrays of a *reading*, is renamed
   **`being::surface_digest`** and its doc says what it hashes; the labels that called it the world's digest
   moved with it, and **the figures on those lines did not** (the `U1 client:` read digest is still
   `88304a75…`): a rename moves no byte of what is hashed. U2's other identity lines are untouched
   (`b6315519…`, `3eb19116…`, `c9e3096e…`), **U9's saves carry no cubic byte**, and **U10's rows are not
   moved**.
3. **The seam: the world model moves first.** The candidates were the mesh (heightfield merge → greedy merge
   over **voxel faces**), the client's camera (plan view → projected), and the world model itself. The world
   model moved, because a mesher or a camera built first would have to invent its own representation of the
   cells — the second world model the tree forbids. `mesh.rs` and `surface.rs` were **not** restructured, and
   both read one thing (`CubicWorld::column`, or the cells) from here on. **The cost is named**: the surface
   is quantised to the voxel, the routing graph read off it differs at **2 033 of 2 304** columns, and the two
   planes settle to different states (2 079 standing, deepest 7 809 µm at column 330, digest `8010a2ea…`
   against the generator-fed plane's 1 647, 441 809 µm at 1271, `19df3015…`) — printed, not required to agree.
4. **The client: `src/surface.rs` is kept, and named a plan-view instrument.** Not retired: it reads the cubic
world's *surface reading* and draws one sample per column. Its windowed lines (**six** in this pass's own
capture, which names the scratch file it wrote), its 39 bands and its **66 355 200 of 66 355 200** pixels read
back are measurements of that reading. **The owed GPU pass is
   checked against that instrument's own projection** — `presentation::shade`'s answer per column, at the
   instrument's own zoom and exaggeration — not against a picture whose shape was refused. A projected camera
   over voxel **faces** is the *later* U1 step, owed.

### What the slice is, and what it deliberately left alone

New: `clean-room/src/chunk.rs` (**736** lines) — the cells, `Cell`, `Chunk`, `CubicColumn`, the build
(`from_surface`, the one admission: the record's stages produce a surface, so a cubic world is filled *from*
one), the identity, the reading, and **seven guards**. Changed: `being.rs` (`surface_digest`'s rename and
`WorldRead::over_cubic`), `evidence.rs` (`cubic_cells`, run **before** the bar is composed), `surface.rs` (the
rename and the doc that names the instrument), `main.rs` (one printed label), `lib.rs` (the module and the
list), `systems.rs` (U2's row), and the two U12 test files (the renamed call).

Left alone, by decision rather than by omission: **nothing under `crates/` was read or written** (the witness
stays a behavioural witness); `mesh.rs`, `surface.rs`'s drawing and the save format were not restructured;
U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score` calibration sets and the score rungs are **owed and
untouched**; and the class's `CELL_HOLD_UM` cap is **not** observable through the reading, which the guard
section below says rather than implies.

### The measured numbers

| figure | value |
|---|---|
| the world's identity | `9d2eea8cd460475b57174ffd8409c4861b70a6dab47f0446746108cb741df265` |
| the same patch, read two ways | cells `97e0ca2f…`; generator's fields `e07145a5…` |
| one place, both readings | column 1271 → (23, 26): 73.5539 m / 127 536 µm → 73.0000 m / 127 536 µm (Δ 0.5539 m) |
| the water, every column | exactly at **2 304 / 2 304** (µm both sides); the *read* answers the cells at **2 299 / 2 304**, never by more than **1 µm** |
| the automaton over the reading | 200 tick(s): **2 079 / 2 304** standing, deepest **7 809 µm** at column 330, `8010a2ea…` |
| the same patch on the column-fed plane | 200 tick(s): **1 647 / 2 304**, deepest **441 809 µm** at 1271, `19df3015…` — **the same state = false**, printed |
| the routing graphs | differ at **2 033 of 2 304** column(s) |
| the client's pixels | the windowed `P6` frame is **byte-identical** to §36's `frame.ppm` (`cmp` clean, 995 343 bytes, `86daeaf3…`); **66 355 200 / 66 355 200** pixels read back |

### The captures, and the movement accounted line by line

`docs/baselines/u2-cubic-cells/`: `pre-stdout.txt` (**510** lines, `3bd01135…`), `final-stdout.txt`
(**514** lines, `b49dbd28…`), `window-stdout.txt` (**6** lines, `4c53a0e8…`), `digests.txt`, `run-guards.sh`,
`proof.txt`. **The pre-capture is §36's own final bytes by digest**: `3bd01135…` is the digest
`docs/baselines/u1-visible-client/digests.txt` records for that baseline's `final-stdout.txt`, so the
movement below is between the record's own two ends rather than between two re-takes.

**510 → 514: four added, four changed, none removed.**

* **changed** — line 13 (`[U2]` status: the cubic cells, their one identity and the reading of them), line 14
  (`unifies:` gains *the cubic chunk's own cells*), line 18 (`interface:` gains
  `chunk::{CubicWorld, Cell, CubicColumn}`), line 99 (`U1 client:`: the label `world cells_digest` →
  `surface read digest`, **every figure on it unchanged**).
* **added** — lines 109–112: `U2 cubic cells:` / `U2 surface read:` / `U2 cubic reading:` / `U2 cubic step:`,
  with the figures in the table above (510 + 4 = 514 ✓).

The windowed capture (`window 200 … frame.ppm`) moved **one label** and **no pixel**; its 6 lines carry the
presented-frame count, the plane at rest, the renamed read digest, the read-back count, the bands and the
screenshot.

### The guards, twelve, proved to bite by name — and the two that are named unexercised

`docs/baselines/u2-cubic-cells/run-guards.sh` perturbs **each new guard in its own subject** and runs the bar:
the named empty, a place outside the patch, the world's own chunk-boundary ceiling, the three-array check, a
class that cannot be formed, the voxel that contains the ground, the water read above the surface, one cell in
the identity, the patch framed in the identity, a pour that is a set of the greater water, and the two lines
of `over_cubic` that source the being's read from the cells. Every case **bit by name** (the test's own name
is in the proof), and every touched file came back **byte-exact by digest** (the harness fails a case whose
restore does not verify). Four tests are new with this pass — `a_class_that_cannot_be_formed_is_refused_by_name`,
`a_patch_one_column_over_is_another_world`, `a_second_pour_does_not_shrink_the_water_of_a_cell` and
`the_reading_a_being_takes_is_sourced_from_the_cells` — and the last of them is the ruling's central claim as
a guard: **one micrometre written into one cell moves the being's reading**, which a read re-derived from the
fields could not do.

The harness's **first attempt at a case for the patch's framing bit nothing**, and that is a finding rather
than a failure to hide: a patch one column over moves every column's *placement*, so the cells' own bytes
differ too and no single-subject perturbation isolates the framing from the cells. The case was replaced by
one that does isolate its subject — the reading's own **column order** — the test it aimed at was renamed for
what it measures (`a_patch_one_column_over_is_another_world`), and the chunk-key and patch-origin framings and
`CELL_HOLD_UM`'s cap are **named unexercised** in `LEDGER.md` C-189 §189.8 instead of being implied to be
proved.

The harness's last check is **the run's own determinism**: it snapshots the binary's output *before* any
perturbation and requires the rebuilt binary to reproduce it byte for byte, then *reports* whether the dated
capture still matches. §36's harness compared against the dated capture, which turns red for the first later
pass that moves one printed label — this pass moved one, and the repair is at the cause rather than by
re-taking that baseline.

### The near-misses this pass found in its own work, and how each was fixed at the cause

1. **Two unit bugs of one kind, both found by running rather than by reading**: the reading's surface was
   printed and compared as **metres against micrometres** (`73 000 000 µm` reported as "72999926.4461 m"), in
   `evidence.rs`'s line *and* in `chunk.rs`'s own test. Both now convert at the boundary; the test that caught
   the second is why the first was caught at all.
2. **A comparison against the wrong state**: the reading line compared the cells (built from the **Tick(0)**
   fields) against the **settled** plane's depth at that column — one world's Tick(0) against another world's
   rest. It now reads the Tick(0) field on the same side and prints the settled figure as what it is: a later
   state.
3. **A slice that could be skipped**: the first cut called the slice inside the bar's `&&` chain, so a failing
   earlier conjunct would have skipped the cubic lines **silently**. It runs before the bar now, and the bar
   reads its answer.
4. **A stale figure in a claim, corrected**: §36's graded dimensions print `src/evidence.rs` as `4 121 lines`
   (two places). The audit measured **4 198** on §36's own last bytes, and this pass leaves **4 378**. Both
   occurrences are corrected in place, dated, and the file's own growth is this pass's own (+180 lines).

### What the slice found that the ruling must live with

The cubic world's surface is its own voxel's, and the generator's is a float: **the routing graph read off the
quantised surface differs at 2 033 of 2 304 columns**, so the game's own water law does not move from one
representation to the other unchanged. Two honest exits are owed and neither is taken here: **(a)** the water
law restated **on the cells** (a graph read from the cells' own ground or faces), or **(b)** the surface's
**remainder carried inside the voxel** (the record's per-cell frame format, itself owed). Also measured and
owed to §34 rather than to this slice: the read's `depth_um` truncates, so 5 of 2 304 columns answer one
micrometre low — printed by the run as *2 299 of 2 304, never by more than 1 µm*.

### The done conditions, with exit statuses

`cargo test --offline` → **404 passed / 0 failed** (exit 0; **318** library + **86** across the thirteen
integration suites (the crate's thirteen `tests/*.rs` files);
393 → 404: **eleven** new guards). `cargo clippy --offline --all-targets -- -D warnings` → exit **0**.
`cargo run --offline` → exit **0**, **514** lines of stdout. The guard harness → exit **0**, twelve cases bit
by name, every file byte-exact. The windowed run → exit **0**, frame byte-identical to §36's. `sha256sum -c`
over `docs/baselines/u2-cubic-cells/digests.txt` verifies the capture pair and the sources this pass touched.

### The four graded dimensions, with the top gap in each

| dimension | grade | what carries it, and the top gap |
|---|---|---|
| **Spec** — the record's own obligations, read as checks | **6/10** | rulings 1–4 are recorded, the shape is landed, and the acceptance checks of the row the shape touches (U2's) are met and printed. **Top gap**: U2's own bar is *not* yet the cubic world's — the row's `T016` axis vocabulary, the `Section`'s owed row, and U2's `in progress` status stand; a ruling is not a completed row. |
| **Design** — one owner per piece of state | **6/10** | one module owns the cells, one function owns the identity, one function owns the reading, one call site hands it out, and nothing reconstructs a second model. **Top gap**: the reading is **recomputed per consumer** (`over_cubic` builds four vectors every call) and `CubicWorld::column` re-walks the vertical span per column, so a whole-patch read is O(chunks × cells) with copies; at 4 chunks it is immaterial, at planet scale it is the difference between a world and a demo. |
| **Correctness** — measured behaviour on the final bytes | **5/10** | the identity is a function of the cells and moves with any of them, the water round-trips at every column, the reading follows the cells, the automaton steps the reading deterministically, and twelve guards bite by name. **Top gap**: **the cubic world's own hydrology is not the world's** — the routing differs at 2 033 of 2 304 columns, so the automaton consumes a reading whose flow law is the *generator's* read on a quantised surface; until (a) or (b) lands, "the same world" is a claim about the cells and not about the water. |
| **Quality** — the tree's own bar: honest refusals, no stale figure | **5/10** | the run prints the disagreement rather than requiring agreement, the two unexercised guards are named, the unit bugs and the skipped-slice defect are recorded as found, and no figure is printed that the pass did not measure. **Top gap**: **the pass's own evidence function grew to 4 378 lines and was not restructured** — `evidence.rs` is now the largest file in the crate and a third pass in a row has added to it; the next pass that touches it should split the per-system slices into the modules they measure, or the file becomes the tree's own unreviewable centre. |

### Owed, and named

1. **The water law on the cells** (§"What the slice found…"): the routing graph read from the cells, or the
   surface's remainder inside the voxel. This is the next seam.
2. **The guards named unexercised**: the chunk-key and patch-origin framings in the identity (the harness's
   framing case **bit nothing** and is recorded as such), and `CELL_HOLD_UM`'s cap, which no reading can
   observe.
3. **U4's mesh**: the heightfield merge becoming a merge over voxel **faces** (`C-43`: *34 816 visible faces
   became 6 quads*), with the resident set keyed by the cells' identity — which ruling 2 has now supplied.
4. **U1's remainder**: the GPU pass against the plan-view instrument, a resizable window, the Sun's azimuth,
   and the projected camera over faces.
5. **U9**: a save of a resident cubic set (the format's `Save::of` is built over a reading).
6. **Untouched and owed, exactly as §36 left them**: U10's rows 8, 11 and 13, `mc-ar-mod`, the
   `noul`/`score` calibration sets, the score instrument's rungs, U2's axis vocabulary, and the `P8` gate
   (U13's).

### The next buildable step

**The water law on the cells**, and the reconnaissance is done: (1) the law lives at one site —
`shapers::water::d8_receivers(patch, &elevation)` — and takes a **per-column float** surface, which is why the
graph moved when the surface quantised; (2) the cubic world already answers everything a cell-level graph
needs (`CubicWorld::cell`, `Chunk::cells`, `column.surface_z`, `column.water_um`), so a face- or
cell-level graph needs no new actor, only a second *reading* of the same cells; (3) the automaton's contract
is `Automaton::over(&WorldRead)`, so a cell-level graph must either arrive **through the same `WorldRead`**
(the receivers field is already a per-column `Vec<Option<usize>>` — the honest first move is to make the
receivers' *derivation* the cells' and keep the struct) or the automaton gains a second constructor, which
would be the second world model this record keeps refusing. The measured acceptance check is already printed:
**the two graphs must agree on the same patch, or the difference must be named and attributed to the cells
rather than to the quantisation** — and today they differ at 2 033 of 2 304.
## 38. The water law on the cubic world's own cells — one law, one cell state, one tick, and §37's open question measured (2026-10-13)

§37 landed the world and left one seam owed **in its own words**: *the water law on the cells*, with exit **(a)**
stated as *a graph read from the cells' own ground or faces*. This pass takes **(a)** and nothing else. The law
that was on the generator-fed column field beside the world now **is** the world: `src/automaton.rs` holds a
`CubicWorld` as its state, reads its routing graph from the surfaces **those cells return on the tick it is
taking**, writes the water back into **those cells**, and keeps nothing else — no per-column depth vector of its
own, no second scheduler, no second needs model. `being::WorldRead::over_cubic` of `Automaton::world()` is
therefore a reading of the world the automaton actually stepped, and there is no second path by which a being
could be handed a plane the law did not produce.

Fresh clean-room code, derived from the documents (§32's sentence, §33's owed line, §37's ruling 3) and from the
crate's own seams — never from the witness. The record orders three things **after** this seam and this pass keeps
them out: the mesher and its height merging (U4's face merge), the GPU pass, and the plan-view client's own
projection. Nothing under `crates/` was read or written.

### One law, one state, one tick: where each concern lives

| concern | owner |
|---|---|
| **the cell state** | `CubicWorld` — the water stands in the cells' own `water_um`; `Automaton` holds that world and the count of ticks it has taken, and **nothing else** |
| **the law** | `Automaton::step` — the declared rain, and one synchronous hop by the graph *read from the cells' surfaces on that tick* (`d8_receivers`, called on what the cells return, never re-derived) |
| **the tick** | `Automaton::step` returns the `Tick` it produced; `Automaton::steps` advances a declared count. **No clock, no thread, no schedule**: the caller's own call is the only time there is |
| **the law's read** | `CubicWorld::water_columns` — every column's ground and the water above it, in two sweeps over the chunks' cells, and the same arithmetic as the one-column reading (a guard measures the two against each other) |
| **the law's write** | `CubicWorld::pour_columns` — the inverse, at most one voxel's worth per cell (`CELL_HOLD_UM`), refusing **by name** a negative depth and water the world's own span cannot carry |
| **the difference the voxel makes** | `attribute_routes` — §37's open question, measured per column and decomposed by cause |
| **the being's own walk** | `being::walk_to_water` — where a walk from a cell first stands in water and how many hops that is; `start_cell` is now one line over it, so the walk is stated once |

### §37's open question, answered by measurement

§37 left it open whether the **2 033 of 2 304** columns that route differently are *a faithful consequence of the
cubic law or a defect in the reading*. It is answered by two measurements, printed by the run, and the answer is
**the voxel's**:

* **The reading itself is the declared quantisation at every column.** `attribute_routes` checks, at **all
  2 304** columns, that the cells' ground is the voxel that contains the generator's float elevation
  (`floor`) — **2 304** are, **0** are not. A reading that quantised wrongly would still decompose into three
  tidy voxel causes, which is why this check is measured **before** the decomposition rather than inferred from
  it; a regression that moved the quantisation turns it red (guard 10 in the harness below).
* **Every difference is one of the three voxel effects.** Of the columns that route differently: **1 970**
  swallowed whole in their own voxel (the generator drains and every neighbour its own law considered strictly
  lower sits in the column's *own* voxel — the whole drop is sub-voxel, so the cells honestly see a pit),
  **0** where the generator's winner is inside its own voxel while the cells route to the steepest candidate
  they can see, **63** tied at the steepest quantised drop (both are candidates and the quantised drops are
  equal, so the law's own scan order decides between equals), **0** wrong way (impossible while the floor is
  monotone, measured rather than assumed) and **0** unexplained (a difference the voxel cannot account for —
  exactly what a defect in the reading would look like).

1 970 + 63 = **2 033**, and the guarded totals are exact: every column is counted once, and the attributed total
equals the direct comparison. So: **a faithful consequence of the declared voxel quantisation, not a defect in
the reading.** What remains owed is the record's own exit **(b)** — carrying the surface's remainder *inside*
the voxel (the per-cell frame format, §36's owed list) — which would move the graphs together at the cause rather
than restate one of them.

### The being's need, on the world the law actually stepped

The law holds the world, so the being is read out of the cells (`being::WorldRead::over_cubic` of
`Automaton::world`) rather than out of a column field beside it, and the run prints what that buys:

| figure | value |
|---|---|
| the world the law starts from | the cubic world of §37: **4** chunk(s) × 32768 = **131 072** cell(s) (**24 689** solid), one identity `9d2eea8c…` |
| the state at rest | **200** tick(s): **2 079 of 2 304** column(s) stand water, deepest **7 809 µm** at column 330, plane total **4 194 364 µm**, state digest `3eb284a3…`, unmoved from tick 200 to tick 700 |
| the first mob | cell **111** of 2 304, **1** step along the game's own flow law, drinks at column 159 (**0.010996 m**) at Tick(0) and at column 159 again (**0.005809 m**) at rest — its need met on both planes |
| the walk, column by column | **174 of 2 304** column(s) walk to water differently on the settled plane than on the plane the being was placed in (reach 8): **174** walk(s) the rain made possible, **0** the drain took away, **0** moved |
| the read against the cells | the read's own whole micrometres answer below the cells' at **0 of 2 304** column(s) at rest, never by more than **1 µm** (at tick 8 the same measurement is **7 of 2 304**) |
| the walk's own answer | at a column the rain made possible, the being is refused `water_unreachable` on the plane it was placed in and drinks on the settled plane, at the column and in the number of hops `walk_to_water` names |

**One measured fact is stated rather than dressed up**: at the being's *own* start cell the walk is
`1 step then drink at column 159` on **both** planes — the route is the ground's and the ground did not move —
while the water it reaches there moved **10 996 → 8 038 µm** (tick 8) and stands **5 809 µm** at rest. §33's
sentence (*a rained-on or drained column changes the being's next step*) is therefore measured **column by
column** rather than read off the one cell the being happens to stand in: 174 columns change, every one of them
because the rain made a walk possible where the generator's plane had none.

### The defect this pass found by running, and the fix at its cause

The world is spanned **whole chunks**, so it owns cells outside the patch's own footprint; the law's three sweeps
indexed the patch's fields by the cells' plan position **without bounding them**, so the first pour walked past
the end of those fields — an index out of bounds, found by the first test that poured, not by reading. The cause
is one missing mapping, and it is fixed once: `chunk::patch_index` answers a cell's column in the patch's own
order **or nothing**, the three sweeps share it, and a cell the world owns but the patch does not name is not a
column of this patch. Guard 9 below perturbs exactly that bound and the harness shows the panic returning.

### The captures, and the movement accounted line by line

`docs/baselines/u2-water-law-on-the-cells/`: `pre-stdout.txt` (**514** lines, digest `b49dbd28…`, which is
**§37's own `final-stdout.txt` by digest**, so the movement below is between the record's own two ends rather
than between two re-takes), `final-stdout.txt` (**517** lines, `ef8b5547…`), `digests.txt`,
`digests-captures.txt`, `run-guards.sh` and `proof.txt`.

**Verified by grep.** A sweep for every figure this pass moved — `144 quads`, `1031 cells`, `merge factor
7.16`, and the twelve digests the movement above lists — over `clean-room/src`, `clean-room/tests`,
`clean-room/specs`, `clean-room/README.md`, this plan and `LEDGER.md` returns **no stale hit outside a dated
block**: every dated block that holds one now carries its own `(§39, 2026-10-13)` marker beside it, and the
remaining hits are this section's and `C-191`'s own quotations of the **before** values (and the earlier,
dated sections of this plan, which are accounts of their own passes).

**514 → 517: three added, twelve changed, none removed.**

* **added** — `U2 route attribution:` (the decomposition, on the `U2` side), and on the `U12` side the
  `U12 stepping walk:` line (the walk measured column by column) and the `U12 route attribution:` line (the same
  decomposition printed where the being's row owns it).
* **changed** — the `[U2]` row (the water law now steps those cells) and the `[U12]` row (one law, one cell
  state, one tick, no second path, and §37's answer printed as a measurement); the two `U1 client:` lines, which
  now read the **cell law's** world (2 079 standing, deepest 7 809 µm at column 330, 20 bands, frame `bbdf3b91…`,
  read digest `70a06acd…`, plane state `3eb284a3…`) instead of the generator-fed plane's — the client is the
  instrument over the world the law steps, and the record's own reading of *which* world that is had to move with
  it; the `U2 cubic reading:` line (its column is the settled cell plane's deepest, 330 → (42, 6)); the `U2 cubic
  step:` line (the law steps the cells themselves, and the reading of that world answers the cells' own water at
  **2 304 of 2 304** columns); the two `U12 first mob:`/`U12 stage question:` lines (the plane it stands in is the
  cells', and the draft digests moved with the bytes); `U12 determinism:` (its two-run chain digest moved with the bytes, `979bf776…` → `806cbae7…`); `U12 settled plane:` (the cells' own state: 2 079 at rest,
  4 194 364 µm, `3eb284a3…` — the same standing set and deepest column §37 printed for the *reading*-fed plane,
  and a **different state digest**, because this law starts from the cells' exact micrometres instead of §37's
  truncating float conversion, which is the movement §38 records); `U12 stepping:` (the plane's own water at the
  drink column, printed as its own value); and `U12 stepping determinism:` (the new state digest, and a different
  settled-chain digest).

### The guards, twelve, proved to bite by name

`docs/baselines/u2-water-law-on-the-cells/run-guards.sh` perturbs **each new guard in its own subject** and runs
the bar; every case **bit by name** (the test's own name is in `proof.txt`) and every touched file came back
**byte-exact by digest**, with the rebuilt binary reproducing the start-of-run bytes and this pass's dated
capture still describing them.

| case | subject | what bit |
|---|---|---|
| the law's own rain | `RAIN_UM_PER_TICK` in the tick's arithmetic | `one_tick_is_one_synchronous_hop_by_the_declared_arithmetic_over_the_cells` |
| the pit's declared fraction | the retention a pit keeps | `rain_wets_a_dry_pit_and_the_plane_comes_to_rest_at_the_laws_own_fixed_point` |
| one synchronous pass | a hop keeps nothing of its own | `one_tick_is_one_synchronous_hop_by_the_declared_arithmetic_over_the_cells` |
| the state's identity is the cells' | the digest's own argument (not the tick count) | `the_plane_comes_to_rest_and_the_settled_world_is_not_the_transient` |
| the graph is the cells' own surfaces | the elevation the law hands its own routing function | `the_law_steps_the_cells_and_not_the_generator` |
| the law's own tick count | the increment | `the_law_is_a_pure_function_of_the_cells_and_the_tick_count` |
| the pour never writes a column's ground | the sweep's own lower bound | `the_pour_round_trips_the_water_it_is_handed` |
| the cell's own cap (`CELL_HOLD_UM`) | the clamp's own bound — **§37 named this unexercised; it has a subject here** | `a_depth_past_one_voxels_worth_spills_into_the_next_cell` |
| a cell outside the patch is not one of its columns | `patch_index`'s bound (**the defect above**) | `the_pour_round_trips_the_water_it_is_handed` |
| the reading is the declared voxel | the build's own quantisation | `every_route_difference_is_the_voxel_and_none_is_unexplained` |
| the walk's own water | where a walk stops | `a_rained_on_column_changes_the_beings_next_step` |
| the walk counts its own hops | the walk's own step arithmetic | the same |

**Two cases had to be strengthened before they could bite, and both are findings rather than harness repairs.**
The graph case's first perturbation — shifting *every* surface by one metre — moved **no** difference and so bit
nothing, because `d8_receivers` compares neighbours and a uniform shift is not a perturbation of the graph at all;
the graph guard now checks the law's arithmetic at **every** column against both graphs and counts the columns
the two answer differently on. The walk case's first perturbation — a longer reach — changed the count on no
column, because the predicate the test asserted was the drink **column** and not the walk's own hop count; the
walk guard now compares the being's own next step as a **pair**, hops and column. The ground bound's own case
required the guard to read the **cells** rather than the columns, because the reading sums only above a column's
ground: water written into the ground itself would be invisible to it while still moving the world's identity.
The two framings §37 named unexercised (the chunk key and the patch origin inside `CubicWorld::digest`) are
**still unexercised**, for §37's own reason, and are named here rather than implied to be proved.

### Where the law's own account lives — and one thing said plainly about the crate's pattern

The law's account lives **with the law**: `src/automaton.rs`'s module header states the law, the state, the tick
and the attribution's meaning, `attribute_routes` carries its own contract, and the guards are that module's own
tests and the two U12 test files. `src/evidence.rs` gained **no prose** — only the instrument lines the run
prints (the decomposition, the walk measurement, and the three corrected lines), which is what that file is for.
This **follows** the crate's pattern rather than departing from it — per-module docs, a shared `evidence.rs` for
the run's own instruments, and the dated account in this plan and `LEDGER.md` — and the one place it does not is
named instead of followed silently: `evidence.rs` is **4 567** lines and §37 left **4 378**, a fourth pass in a
row to add to it, and this pass's own contribution to that growth is **+189** lines of instrument rather than of
prose. The split §37's top gap names is now overdue.

### The done conditions, with exit statuses

`cargo test --offline` → **411 passed / 0 failed**, exit **0** — **325** library tests (318 at §37: **+7**, the
law's own six new guards plus the widened routing guard) and **86** across the thirteen integration suites.
`cargo clippy --offline --all-targets -- -D warnings` → exit **0**. `cargo run --offline` → exit **0**, **517**
lines of stdout, *"Clean room OK"*. The guard harness → exit **0**, twelve cases bit by name, every file
byte-exact, the rebuilt binary reproducing the start bytes, and the dated capture still matching (`proof.txt`).
`sha256sum -c` over `docs/baselines/u2-water-law-on-the-cells/digests.txt` (run from `clean-room/`) verifies the
sources this pass touched, and `digests-captures.txt` (run from the baseline directory) the pair.

### The four graded dimensions, with the top gap in each

| dimension | grade | what carries it, and the top gap |
|---|---|---|
| **Spec** — the record's own obligations, read as checks | **7/10** | §37's owed item 1 (exit **(a)**) is landed as fresh code with one owner, and the sentences it closes (§33's *state that carries tick to tick*, §34's law) are measured rather than restated. **Top gap**: exit **(b)** is untouched, so the world's own hydrology still differs from the generator's at the quantisation — the law is honest about the difference, it does not remove it. |
| **Design** — one owner per piece of state | **7/10** | one struct holds the world and the tick count and nothing else; the graph is not stored at all (read per tick from the cells); the read and the write have one owner each and are inverses; the being's walk is stated once and `start_cell` is one line over it. **Top gap**: `Automaton::over` **clones the world whole** (every chunk, every cell) and `pour_columns` re-sweeps every cell of every chunk each tick — immaterial at 4 chunks, the difference between a world and a demo at planet scale. |
| **Correctness** — measured behaviour on the final bytes | **7/10** | §37's open question is answered by measurement (2 033 differ, all voxel effects, 0 unexplained, the reading the declared voxel at 2 304 of 2 304), the tick's per-column arithmetic is checked against both graphs at every column, the state is a pure function of the cells and the count, the water round-trips exactly, and twelve guards bite by name. **Top gap**: the **equilibrium** is a declared choice rather than a derived one — the pit's retention is what bounds the plane, and nothing here tests that a *different* retention would move the settled shape in the way the record would want. |
| **Quality** — the tree's own bar: honest refusals, no stale figure | **7/10** | the run prints the difference and the attribution's two zero-counts rather than requiring agreement; the walk is measured column by column rather than asserted at one cell; the defect found by running is recorded with its cause; two guards were strengthened because their cases bit nothing; and the two still-unexercised framings are named. **Top gap**: **`evidence.rs` is 4 567 lines** and this pass added 189 of them — the fourth pass in a row to do so, and the split §37 named is still not taken. |

### Owed, and named

1. **Exit (b) — the surface's remainder inside the voxel** (the per-cell frame format, §36's owed list): the
   only remaining way the two graphs move together at the cause.
2. **The framings named unexercised** (unchanged from §37): the chunk key and the patch origin inside
   `CubicWorld::digest`.
3. **`evidence.rs`'s split** — the per-system slices moved into the modules they measure.
4. **U4's mesh** (the heightfield merge → a merge over voxel **faces**, keyed by the cells' identity), **U1's
   GPU pass** against the plan-view instrument, a resizable window and the Sun's azimuth, **U9's** cubic save,
   and **U13's** `P8` gate.
5. **Untouched and owed, exactly as §36 and §37 left them**: U10's rows 8, 11 and 13, `mc-ar-mod`, the
   `noul`/`score` calibration sets, the score instrument's rungs, and U2's axis vocabulary.

### The next buildable step

**U4's mesh, on the cells the law now steps** — §37's reconnaissance stands (the faces between a solid cell and
air are what a merge over the cubic world is, and `chunk.rs` already owns them), and this pass adds one fact to
it: the resident set's key can be the cells' identity *while the water is moving*, because the law's own state
digest **is** the world's identity (`Automaton::digest` is `CubicWorld::digest`) — so a mesh keyed on a stepped
world's identity is re-keyed exactly when a cell's water moves, which is the invalidation the record asks for.
The mesh must not keep the heightfield merge beside a face merge: one mesher, one source, the column path retired
or re-expressed as a reading of the same cells.
## 39. The mesh over the cells' own voxel faces — one owner for the faces, the merge and the mesh's own state, and the column merge kept as a reading (2026-10-13)

§38 landed the water law on the cells and left one seam owed **in its own words**: *"the mesh must not keep the
heightfield merge beside a face merge: one mesher, one source, the column path retired or re-expressed as a
reading of the same cells."* This pass takes that seam and nothing else. A mesh is now the world's **own voxel
faces** — the face between a solid cell and air, and a column's water top where the cell above holds none —
merged **greedily** into rectangles by the one mesher in `src/mesh.rs`, whose resident key is the **cells' own
state digest**. The user has ruled the column shape the product rejects, so a mesh that merges column **heights**
would be part of that refused shape; the merge's unit is therefore a **face**, and the height merge is **kept and
named a reading** rather than left standing as a second mesh path.

Fresh clean-room code, derived from the record's own documents (`docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5, slice 4
of the plan's §5, `LEDGER.md` C-43 and C-46, and `specs/004-u4-mesh-stream-work/contracts/u4-seams.md` C1–C4) and
from the crate's own seams — never from the witness. The record orders four things after this seam and this pass
keeps them **out**: the **GPU pass**, the **shader work**, the **plan-view client's own rendering** and the
**second being**. Nothing under `crates/` was read or written.

### One owner for the faces, the merge and the mesh's own state

| concern | owner |
|---|---|
| **the faces** | `Mesh`'s one sweep over the cells a chunk's footprint covers: a **side** is a face where the neighbour is not solid, and a **column's water** shows its top where the cell above holds none — the world's own quantisation is therefore the geometry's, and a step in the ground is a wall rather than a new rectangle |
| **the merge** | one greedy pass keyed by `(layer · axis · facing · plane · material)`: rows grow first, then whole rows, and `merged_away` counts **only rectangles that grew** — so the accounting cannot claim a saving the merge did not make |
| **the mesh's own state** | `Mesh { key, quads }` — the key, the rectangles, and the vertex/triangle counts **read off them** (`4 × quads`, `6 × quads`), not stored beside them |
| **the resident key** | `GeometryKey { state, chunk, level }` — U2's truth digest for the reading's door, and the **cells' identity** (`CubicWorld::digest`) for a cell world, which **is** the law's own state digest (`Automaton::digest` is `CubicWorld::digest`) |
| **the reading** | `Mesher::derive` — the declared fields read in one evaluation, decimated to the level's tile grid, filled by the one constructor (`from_surface`) and swept: the **column merge, kept and named a reading** |
| **the cells' own door** | `Mesher::derive_cubic` — a world that already holds cells is swept directly, the level ladder refuses to coarsen it **by name**, and the key is its own state digest |
| **the work accounting** | `WorkLedger` — every counter incremented where the work happens, and the whole ledger checked against an **independent recount** of the same cells (`recount_faces`, which shares no code with the sweep) |

### The old path: **kept, and named a reading** — not retired, and not left standing

The height merge is the record's own earlier mesh and this pass **keeps** it, renamed by what it is: the run
prints it as `U4 reading:` — *the declared field plane, read and merged* — beside `U4 cells:`, the mesh over the
world's cells. **There is one mesher and one merge.** The two doors differ in *what they read* (a declared field
plane, versus a world that already holds cells), not in how they merge or in what a mesh is, and both answer the
same `Mesh` type, the same ledger and the same recount. Nothing in this tree can now build a mesh by merging
column heights and call it the world's geometry: a face merge is the mesh, and a height merge is a reading of a
plane. The record says which is which **in the line it prints**, so a reader never has to infer it.

### What the run prints, and what each number is of

| figure | value |
|---|---|
| the reading (seed `1592639215`, period `1 s`, tick `0`, chunk `(0, 0, 0)`, level `0`) | `1024 columns read (1024 samples) → 11054 cells swept → 3703 faces → 3297 merged away → 406 quads (259 merged, factor 9.12)`, **1 624** vertices, **812** triangles, digest `2db62b78…b79c`, keyed by the reading's own state `146ff7bf…381733` |
| the recount of the same cells | `1024` column(s) swept, **11 054** solid cell(s), **3 596** solid + **107** water faces, **3 297** merged away into **406** quads carrying **3 703** faces of area — equal to the ledger = `true` |
| the worker knob | one digest `2db62b78…b79c` at **1/2/4/8** workers = `true` |
| the cache | a re-ask is a hit producing **0** more columns, **0** more faces and **0** more quads; at tick 1 the tick-0 entry is **not** served (never stale); the rebuild's digest moved (`3748db3d…`); invalidation removed **1 of 2** and kept the current one |
| the coarse level (`level 2`) | read **64** of 1024 produced columns, `11168` cells swept → `3680` faces → **106** quads, digest `12e94e27…e786f` — a level is part of the identity |
| **the cells' own door** (the law's world: patch `48×48`, 2 304 column(s), 1 024 under this chunk) | keyed by the cells' state `9d2eea8c…f265` → `11054` cells swept → **3 047** faces (**2 930** solid + **117** water) → **2 648** merged away → **399** quads (**250** merged, factor **7.64**), **1 596** vertices, **798** triangles, digest `1172ac59…864c`; the recount of the same cells agrees = `true` |
| **the same door after one tick of the law** | state `19fc6cb3…e820` **moved** = `true`; the older state's entry served for it = `false` (never stale), `stale = 1`; rebuild: `11054` cells swept → **3 854** faces (**2 930** solid + **924** water) → **3 443** merged away → **411** quads, **1 644** vertices, **822** triangles, digest `4c2850a8…b556`, keyed by the new state = `true`; **the same ground faces** = `true`; invalidate removed **1**, kept **1** |

**The state digest the resident set is keyed by is the law's own** — `9d2eea8c…f265` before the tick and
`19fc6cb3…e820` after it — so a mesh of the law's world is re-keyed **exactly** when a cell's water moves, with
**no second world model**: the key is derived from the same `CubicWorld` the law steps, and `Resident::serve`
cannot hand back geometry from an older state because there is no key to find it under. One tick of rain moved
the water's faces (**107 → 924**) and left the ground's faces unchanged, which is printed rather than argued.

### The captures, and the movement accounted line by line

`docs/baselines/u4-faces-over-the-cells/`: `pre-stdout.txt` (**517** lines, digest `ef8b5547…`, which is
**§38's own `final-stdout.txt` by digest**, so the movement below is between the record's own two ends),
`final-stdout.txt` (**519** lines, `ecd10f08…`), `digests.txt`, `digests-captures.txt`, `run-guards.sh` and
`proof.txt`.

**517 → 519: two added, twenty-two changed, none removed.**

* **added** — the two `U4 cells …` lines: the cells' own door's mesh (`U4 cells seed=…: world patch 48×48 … keyed
  by the cells' state 9d2eea8c…f265 → 11054 cells swept → 3047 faces … 399 quads … digest 1172ac59…864c`) and
  the same door **after one tick of the law** (`state 19fc6cb3…e820 moved = true; … the same ground faces =
  true`).
* **changed (22)** — the six `U4` lines the seam owns: the reading line (was the height-merge line: `144 quads
  from 1024 columns (1031 cells, merge factor 7.16), vertices 576, indices 864`; now the reading's own counts and
  the keying state), the recount line (`U4 recount:` → `U4 face recount:`, over the same cells), the worker-knob
  line (the new digest), the cache line (now counting **faces** as well as columns and quads), the `level 2` line
  (now sweeping cells and faces) and the owed line (`U4 owed after this seam:` — water's own side faces added to
  the owed list, since this seam shows a water column's **top** face and declares its sides its ground's faces);
  **fourteen** `U6` lines — the `geometry` answer (`144 quads over 1031 cells` → `406 quads over 3703 faces`,
  `mesh=2db62b78…b79c`, answer digest `ffde9d44…`), its `mesh_is_held` claim, its effect line (`-> 406 quads`,
  event `a88bae4e…`), the `held` answer and claim (resident digest `3167bb20…`), the `record` answer and claim
  (head `a88bae4e…`), the `save` answer and the `load` call and answer (the document's own digest `29f521b2…`),
  the `save`/`load` cross-check line, the repeat line (`f6d66a51…`), the record line (`a994e069…`) and the
  transport's repeat line (`723bab9b…`) — all of them answers that **name** the mesh or the record, and therefore
  moved with the mesh; the `U10 documents:` line, whose U4 quickstart digest moved `481412a5…` → `b2bad610…`
  (below); and the `U8 frame digest:` line (`79f3885c…` → `07adeda4…`), because the shell's geometry pane prints
  the mesh's own numbers (`chunk (0,0,0): 3703 faces → 406 quads`, `11054 cells swept, factor 9.12`) and the
  frame's digest is a digest **over the drawn characters**.

### The defect the cascade found by running, and the fix at its cause

The U10 guard `the_documents_the_rows_cite_are_the_ones_on_the_disk` **refused this pass's own edit**:
`clean-room/specs/004-u4-mesh-stream-work/quickstart.md` is the acceptance document the tree's rows declare **by
digest**, and §39's dated note moved its bytes (`481412a5…` → `b2bad610…`), so the declared
`U4_QUICKSTART_DIGEST` in `gameplay.rs` no longer described the document on disk. It was found **by running** the
suite, not by reading it. The cause is one constant, and it is fixed at that cause: the declared digest is moved
to the document's own bytes, the guard is named where it bit (`specs/007-u10-gameplay-tree/quickstart.md`), and
the printed `U10 documents:` line moved with it — which is exactly the read-in-place rule §28/§29 built, doing its
job on this pass's own work.

### The guards, twelve, proved to bite by name

`docs/baselines/u4-faces-over-the-cells/run-guards.sh` perturbs **each new guard's own subject** and runs the
bar; every case **bit by name** (the test's own name is in `proof.txt`) and every touched file came back
**byte-exact by digest**, with the rebuilt binary reproducing the start-of-run bytes **byte for byte** (digest
`ecd10f08…`, exit `0`).

| case | subject in `src/mesh.rs` | what bit |
|---|---|---|
| the side rule (a neighbour that is not solid) | the visibility test itself | `a_flat_uniform_chunk_is_six_faces` |
| the water's own surface (the cell above holds none) | the water test | `two_cells_merge_only_when_layer_plane_and_material_agree` |
| the merge's group key (layer · axis · facing · plane · material) | the key | `a_step_is_a_wall_and_not_a_hole` |
| the merge counts only rectangles that grew | the `> 1` in the merged-away count | `merge_tiles_every_group_exactly_whatever_the_pattern` |
| the columns the world owns (the cells' own patch) | `columns_hold`'s bound | `a_chunk_with_no_column_under_the_world_is_refused_by_name` |
| **the key is the cells' own identity** | `GeometryKey::of_state` at the cells' door | `the_laws_own_step_re_keys_the_mesh_and_moves_no_ground_face` |
| the render ladder does not coarsen a cell world | the level test | `a_level_above_zero_on_a_cell_world_is_refused_by_name` |
| the ledger's face count is the sweep's | the face accumulator | `the_ledger_equals_an_independent_recount` |
| the water face is the ladder's declared water | the water material in the emitted face | `two_cells_merge_only_when_layer_plane_and_material_agree` |
| the quad ceiling is checked after the merge | the ceiling's own comparison | `a_budget_that_cannot_be_met_is_refused_with_its_numbers` |
| a NaN surface is refused with its own field | the finiteness check | `a_stages_refusal_passes_through_the_mesh_seam` |
| the reading's key is the world's own truth | `GeometryKey::new` at the reading's door | `a_stale_entry_is_rebuilt_and_never_served` |

**One thing is said plainly rather than implied**: these twelve cases perturb `src/mesh.rs`'s own subjects and
observe **that module's** guards bite. The end-to-end acceptance tests (`tests/u4_geometry.rs`, three tests driven
through the public API: the reference world's derivation and its ledger, the coherence between the world and its
geometry, and the law's own world through the cells' door) are **not** perturbed by these twelve cases; their
subjects are the same seams plus the binary's own printing, and the harness's final case is the binary's own bytes
byte for byte. That is a named gap with a named cover, not a claim of equivalence.

### §38's own twelve-case harness, re-run on these bytes

§38's harness had not been re-executed since its sources moved, and one of the files it covers **had** moved:
`src/evidence.rs` (`7bf46a73…` → `2e5e5aea…`, this pass's instrument lines). It was re-run on this pass's final
bytes and it is **clean**: exit `0`, twelve cases bit by name (34 fault lines across the twelve cases), every
touched file back **byte-exact** by digest, and the rebuilt binary's stdout and stderr equal to its own
start-of-run capture **byte for byte** (`ecd10f08…`). What it says about §38's dated pair is printed by the
harness itself: *"the dated capture final-stdout.txt describes older bytes: a later pass moved a printed line —
that is the capture's business, not this check's"* — §38's `pre-stdout.txt`/`final-stdout.txt` are **unchanged on
disk** and still verify against its `digests-captures.txt`, and the re-run wrote nothing into §38's captures (its
own proof is the log kept outside the tree; a copy is kept as
[`baselines/u2-water-law-on-the-cells/proof-2026-10-13-rerun.txt`](baselines/u2-water-law-on-the-cells/proof-2026-10-13-rerun.txt),
with a dated block appended to that baseline's `digests.txt`). So §38's seam is shown to bite on the bytes this
tree now holds — the guard is not a fossil of the pass that wrote it.

### Where the mesh's own account lives

The account lives **with the mesh**: `src/mesh.rs`'s module header states the three properties the module exists
to make checkable (the faces are the world's own voxels', a kept mesh cannot drift, the work accounting is real),
the two doors and the one merge; the guards are that module's own tests (**53** test functions) and
`tests/u4_geometry.rs`. `src/evidence.rs` gained **no prose** — only the instrument lines the run prints — which
**follows** the crate's pattern rather than departing from it; the one departure-shaped fact is named instead of
followed silently: **`evidence.rs` is 4 803 lines**, up from §38's 4 567 (**+236**), the fifth pass in a row to
grow it, and the split §37 named is now overdue rather than merely due.

### The done conditions, with exit statuses

`cargo test --offline` → **416 passed / 0 failed**, exit **0** — **329** library tests (325 at §38) and **87**
across the integration suites (86 at §38, `u4_geometry` gaining its third test). `cargo clippy --offline
--all-targets -- -D warnings` → exit **0**. `cargo run --offline` → exit **0**, **519** lines of stdout,
*"Clean room OK"*. This pass's guard harness → exit **0**, twelve cases bit by name, every file byte-exact, the
rebuilt binary reproducing the start bytes. §38's harness → exit **0** on these bytes, twelve cases bit by name.
`sha256sum -c` over `docs/baselines/u4-faces-over-the-cells/digests.txt` (run from `clean-room/`) verifies the
sources this pass touched, and `digests-captures.txt` (run from the baseline directory) the capture pair and the
harness's own proof.

### The four graded dimensions, with the top gap in each

| dimension | grade | what carries it, and the top gap |
|---|---|---|
| **Spec** — the record's own obligations, read as checks | **7/10** | §38's owed sentence is closed on its own terms: the mesh's unit is the face, one mesher, one merge, and the column path is **kept and named a reading** rather than left standing. FR-002's one-digest-per-`(state, chunk, level)` now holds for a **moving** world (the cells' own state digest), and the four refusals (`geometry_budget_exceeded`, `geometry_out_of_residency`, `horizon_mixed`, and a cell world refusing a level above zero by `parameter_out_of_range`) are exercised on the final bytes, with a refusal shown to cache nothing. **Top gap**: U4's slice-4 check also asks for the **per-chunk memory figure**, and it is still **unchecked** — the run says so in its owed line rather than estimating bytes from a struct. |
| **Design** — one owner per piece of state | **7/10** | one owner for the faces, one for the merge, one for the mesh's key and counts, one ledger, one recount that shares no code with the sweep; the resident key is derived from the world the law steps, so **no second world model** exists to drift. **Top gap**: `derive_cubic` sweeps **every** cell of every chunk in the footprint on every rebuild and `Resident` holds whole meshes — immaterial at 4 chunks, and the difference between a demo and a planet at planet scale. |
| **Correctness** — measured behaviour on the final bytes | **8/10** | every count in the run is checked against an **independent recount** of the same cells; the merge's accounting is checked not to claim a saving it did not make; a mesh is re-keyed on a **moved micrometre** (one tick of rain: `9d2eea8c…f265` → `19fc6cb3…e820`, the older entry refused, `stale = 1`); the ground's faces are shown unmoved by the same tick; and twelve guards bite by name on the final bytes. **Top gap**: the geometry is not **stitched** between chunks, so a face on a chunk boundary is emitted per chunk with no cross-chunk merge — the seam is declared, not measured. |
| **Quality** — the tree's own bar: honest refusals, no stale figure | **7/10** | the run prints the reading **and** the cells' own door side by side and names which is which; the recount is independent; the owed line names water's side faces; the U10 digest guard that refused this pass's own document edit is recorded with its cause and fixed there; §38's harness was re-run rather than assumed; and the twelve hand-written cases' **scope** (mesh's own subjects) is named rather than implied. **Top gap**: **`evidence.rs` is 4 803 lines** and this pass added 236 of them — the fifth pass in a row, and the split is now overdue. |

### Owed, and named

1. **Water's own side faces** — this seam emits a water column's **top** face and declares its sides its ground's
   faces (printed in the owed line); carrying the water's sides needs the per-cell frame format's remainder.
2. **The per-chunk memory figure** (slice 4's second half), still `unchecked` and printed as such.
3. **The stitched multi-chunk mesh** and the seam's marching rule, and the **displaced (256×) surface** — a
   rectangle of a displaced surface is not planar and must be subdivided.
4. **The GPU pass** over the plan-view instrument, the resizable window, the Sun's azimuth, **U9's** cubic save,
   **U13's** `P8` gate — kept out of this pass by the record's own order.
5. **Exit (b) — the surface's remainder inside the voxel** (§36/§38's owed list): the only remaining way the two
   graphs move together at the cause.
6. **The framings named unexercised** (unchanged from §37/§38): the chunk key and the patch origin inside
   `CubicWorld::digest`.
7. **`evidence.rs`'s split** — the per-system slices moved into the modules they measure.
8. **Untouched and owed, exactly as §36–§38 left them**: U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score`
   calibration sets, the score instrument's rungs, and U2's axis vocabulary.

### The next buildable step

**The GPU pass over the plan-view instrument** — U1's owed item, checked against `presentation::shade` and the
instrument's own projection rather than against a picture whose shape was refused (§37's ruling 4) — and then the
**stitched multi-chunk mesh** over the same faces, which needs the cross-chunk face visibility rule (a face on a
chunk's boundary is currently emitted by both chunks) before it can claim one geometry for a world.
## 40. One declared boundary-and-span contract, and the stitch across two chunks — a chunk's geometry is no longer an artifact of its world's extent (2026-10-13)

§39 landed the mesh over the cells' own faces and left one gap **in its own words**, as the top item of its
Correctness dimension: *"the geometry is not stitched between chunks, so a face on a chunk boundary is emitted
per chunk with no cross-chunk merge — the seam is declared, not measured"*; and the audit that pass drew on
measured the same **11 054 swept cells** giving **3 703** faces through the reading's door and **3 047** through
the cells' door — a disagreement about **what extent was meshed**, not about the faces. This pass takes that
seam and nothing else: one declared boundary-and-span contract (`Footprint`), the reading's door reading the
ring that decides its faces, and the stitch across two chunks built as **one footprint over a rectangle**. The
audit's disagreement is settled — under the one rule, with the numbers that decided it — rather than explained
away.

Fresh clean-room code, derived from the record's own documents (`docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5, slice 4
of the plan's §5, `LEDGER.md` C-43/C-46, and `specs/004-u4-mesh-stream-work/contracts/u4-seams.md` C1–C4) and
from the crate's own seams, never from the witness. Kept **out** by the brief: the **GPU pass**, the **shader
work**, the **plan-view client's own rendering**, the **second being** and the **module split of
`src/mesh.rs`**. Nothing under `crates/` was read or written; U10's rows 8, 11 and 13, `mc-ar-mod`, the
`noul`/`score` sets and the score rungs are untouched and owed.

### One declared contract, and who owns each part of it

| concern | owner |
|---|---|
| **what is meshed** | `Footprint { patch, span }` — a patch of columns **and the band those columns imply** (`lowest_ground.div_euclid(CHUNK_SIDE) * CHUNK_SIDE` out to `(highest_cell.div_euclid(CHUNK_SIDE) + 1) * CHUNK_SIDE`), so an extent is a function of its own columns rather than of its world's size |
| **what decides its faces** | `MESH_RING` = one cell: the ring the visible surface can be a boundary with; `Footprint::halo_patch()` is the patch the **reading** reads, and `Footprint::halo_owned(world)` is the declaration **with a number** |
| **the interior by declaration** | `solid_at`'s first answer: the footprint's own floor is interior (the band below holds the ground this band stands on), and a position the world does not own is **air** — named, not assumed |
| **the world's own reach** | `Footprint::over` — the **one** place a footprint with no column of the world under it is refused by name (`geometry_outside_the_world`, with the world's patch in the detail); this pass **removed** the second, shadowing count `faces_of` used to carry |
| **the stitch** | `Mesher::derive_region`: one footprint over a rectangle of chunks, one sweep, one merge. A boundary **inside** the rectangle is the world's own inside, so no face is drawn there |
| **the doors** | `Mesher::derive` (reads the chunk **and its ring**), `Mesher::derive_cubic` (a world that already holds cells), the stitch, and the one `merge_into`: one owner for the faces, the merge and the mesh's own state |

### The audit's disagreement, settled with the numbers that decided it

The audit measured two numbers for one sweep, and neither was wrong about its own world:

| door | the world it drew | faces |
|---|---|---|
| the reading's, before this pass | the chunk **alone** (1 024 columns, its own edge drawn as a cliff) | **3 703** |
| the cells', before this pass | the law's own patch (48×48: the chunk plus two of its four ring sides) | **3 047** |
| both, now, over **one world** — the chunk and its ring | `footprint 32×32 + ring 1 (1156 columns, 1156 produced, 1024 samples read), span 64..96`, `halo owned = true` | **1 289** each, and the **same list, byte for byte** (313 quads) |

`U4 extent contract …` prints the three worlds side by side, which is what makes the old pair readable rather
than mysterious: **2 679** faces for a world that is only the chunk (`halo owned = false` — the old reading's
cliff), **1 289** for the chunk + one ring (`halo owned = true`, the same list as the reading's = `true`), and
**2 023** for the law's own world (`halo owned = false`: it owns this chunk's east and north ring columns and
not its west and south). The rule the doors are now held to is one sentence: **faces are decided by a footprint
and the world it is read from** — so two doors agree exactly when they read the same cells, and where they do
not, the run prints which extent each used.

### What the run prints, and what each number is of

| figure | value |
|---|---|
| the reading (seed `1592639215`, tick `0`, chunk `(0,0,0)`, level `0`) | `footprint 32×32 + ring 1 (1156 column(s) in the halo, 1156 produced, 1024 samples read), span 64..96 (1 band(s), 32768 cells), halo owned = true; 11054 cells swept → 1289 faces → 976 merged away → 313 quads (167 merged, factor 4.12)`, **1 252** vertices, **626** triangles, digest `44a682e1…4ffb` |
| the recount of the same cells | `1024` column(s), **11 054** solid cell(s), **1 177** solid + **112** water faces, **976** merged away into **313** quads carrying **1 289** faces of area — equal to the ledger = `true` |
| **one world, two doors** | the same chunk of the world the reading reads, through both doors = `true`: the reading swept `11054` cells in `1289` faces, the cells' door `11054` in `1289`, and their rectangles are **the same list, byte for byte** (`313` quads); only the name differs — `146ff7bf…381733` against `c5ef91c3…bc919` |
| the extent contract | **2 679** / **1 289** / **2 023** faces in three worlds (above), with each world's `halo owned` printed |
| the region (origin `(0,0,0)`, a 2×2 rectangle of chunks) | `footprint 64×64 span 64..96 (1 band(s), 131072 cell(s)) → 44187 cells swept → 8013 faces → 6786 merged away → 1227 quads`, **4 908** vertices, digest `969a29a6…007f`; the shared boundaries at `x=32` and `y=32` carry **6** rectangle(s), the cliffs the two sides really show; the 4 chunks carry the same **6** on those planes = `true`; the 4 chunks meshed one by one draw **8 013** faces, the region **8 013** = `true` |
| the worker knob | one digest `44a682e1…4ffb` at **1/2/4/8** workers = `true` |
| the cache | a re-ask is a hit producing **0** more columns, faces and quads; at tick 1 the tick-0 entry is **not** served; the rebuild's digest moved (`fb314be0…58f4d`); invalidate removed **1 of 2** |
| the coarse level (`level 2`) | read **64** of `1156` produced columns, `11168` cells swept → `1280` faces → **68** quads, digest `060784a0…7a0f` |
| the cells' own door (the law's world, patch `48×48`, 1 024 under this chunk) | footprint `32×32 span 64..96 … halo owned = false` → `11054` cells → **2 023** faces (**1 906** solid + **117** water) → **1 696** merged away → **327** quads (factor **6.19**), digest `5ef04729…dbe0` |
| the same door after one tick of the law | state `19fc6cb3…e820` moved = `true`; rebuild `11054` cells → **2 830** faces (**1 906** solid + **924** water) → **2 491** merged away → **339** quads, digest `36261871…a1a5`; the same ground faces = `true` |
| the budget refusal | `1156` columns (the reading's own produced count) against a declared `64` |

### The captures, and the movement accounted line by line

`docs/baselines/u4-the-extent-contract/`: `pre-stdout.txt` (**519** lines, digest `ecd10f08…`, which is
**§39's own `final-stdout.txt` by digest**, so the movement below is between the record's own two ends),
`final-stdout.txt` (**522** lines, `6c77b9ad…`), `digests.txt`, `digests-captures.txt`, `run-guards.sh` and
`proof.txt`.

**519 → 522: three added, twenty-six changed, none removed.**

* **added (3)** — `U4 one world, two doors:` (the two doors compared face for face over one world), `U4 extent
  contract …` (the same footprint in three worlds) and `U4 region …` (the stitched rectangle).
* **changed (26)** — the U4 block: the reading line (now the footprint, the halo and the span, `1289` faces →
  `313` quads, digest `44a682e1…`), the recount line (`1177` solid + `112` water faces, `1 289` faces of area),
  the worker knob, the cache line (rebuild digest `fb314be0…`), `level 2` (`64 of 1156`, `1280` faces, `68`
  quads), the cells line (the footprint, `halo owned = false`, `2023` faces → `327` quads, digest `5ef04729…`),
  the cells-after-tick line (`2830` faces → `339` quads, digest `36261871…`), the budget refusal (`1156`
  columns) and the owed line (the multi-chunk stitch → **a stitched mesh wider than one rectangle**, because
  the stitch now lands); **fifteen** U6 lines (the `geometry` answer `313 quads over 1289 faces`,
  `mesh=44a682e1…`, answer digest `68f985de…`; its `mesh_is_held` claim and its `event_is` `2ffdd09d…`; the
  effect line `1156/1024 columns -> 313 quads`; the `held` answer and claim, resident digest `3cc0dc04…`; the
  `record` answer and claim, head `2ffdd09d…`; the `save` answer and the `load` call and answer, document
  digest `83c20c30…`; the `save`/`load` cross-check; the repeat `cb7e4eef…`; the record line `510be458…` and
  the transport's repeat `c282e4af…`); the `[U4]` **status sentence** in the systems table (`the multi-chunk
  stitch` → `a region's own residency rather than its one-footprint stitch`); the `U10 documents:` line (U4
  quickstart `b2bad610…` → `ca00f1be…`); and the `U8 frame digest:` line (`07adeda4…` → `375c5122…`), because
  the shell's geometry pane prints the mesh's own numbers (`chunk (0,0,0): 1289 faces → 313 quads`) and the
  frame's digest is a digest **over the drawn characters**.

### The guards, twenty-two, proved to bite by name

`docs/baselines/u4-the-extent-contract/run-guards.sh` perturbs **each guard's own subject** and runs the bar;
every case **bit by name** — the test's own name, or the named line's own text flipping to `= false` — and
every touched file came back **byte-exact by digest**, with the rebuilt binary reproducing the start-of-run
bytes **byte for byte** (`6c77b9ad…`, exit `0`). Its own proof is `proof.txt`: **22 cases, 51 fault lines,
exit 0**.

| case | subject | what bit |
|---|---|---|
| the side rule (a neighbour that is not solid) | the visibility test, now taking the footprint | `a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business` |
| the water's own surface (the cell above holds none) | the water test | `two_cells_merge_only_when_layer_plane_and_material_agree` |
| the merge's group key (layer · axis · facing · plane · material) | the key | `a_step_is_a_wall_and_not_a_hole` |
| the merge counts only rectangles that grew | the `> 1` in the merged-away count | `merge_tiles_every_group_exactly_whatever_the_pattern` |
| **the columns the world owns** | **`Footprint::over`'s own refusal** (the shadow count in `faces_of` removed this pass) | `a_chunk_with_no_column_under_the_world_is_refused_by_name` |
| the key is the cells' own identity | `GeometryKey::of_state`, the cells' door | `the_laws_own_step_re_keys_the_mesh_and_moves_no_ground_face` |
| the render ladder does not coarsen a cell world | the cells' door's level arm | `a_level_above_zero_on_a_cell_world_is_refused_by_name` |
| the ledger's face count is the sweep's | the face accumulator | `the_ledger_equals_an_independent_recount` |
| the water face is the ladder's declared water | the water material in the emitted face | `two_cells_merge_only_when_layer_plane_and_material_agree` |
| the quad ceiling is checked after the merge | the ceiling's own comparison | `a_budget_that_cannot_be_met_is_refused_with_its_numbers` |
| a NaN surface is refused with its own field | the finiteness check | `a_stages_refusal_passes_through_the_mesh_seam` |
| the reading's key is the world's own truth | `GeometryKey::new`, the reading's door | `a_stale_entry_is_rebuilt_and_never_served` |
| **the reading reads the ring it declares** | the halo patch the reading builds its world over | `a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business` |
| **`halo_owned` tells the truth** | the ring's membership answer | `a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business` |
| **the footprint's own floor is interior by declaration** | `solid_at`'s first answer (a **product** case) | `U4 face recount: … = false` (`1 289` against `2 313`) |
| **the span's ceiling is a band above the highest cell** | the ceiling in `Footprint::over` | `one_footprint_stitches_chunks_and_its_faces_ignore_the_world_beyond_the_ring` |
| **a region is one footprint over the rectangle** | `build_region`'s declared side | `one_footprint_stitches_chunks_and_its_faces_ignore_the_world_beyond_the_ring` |
| **the cells' door meshes the chunk's own footprint** | the footprint `faces_of` sweeps | `a_step_is_a_wall_and_not_a_hole` |
| **the two doors are compared over the reading's own world** | the extent one of the doors reads (a **product** case) | `U4 one world, two doors: … = false`, exit 1 |
| **both sides of a region's seam name one world plane** | the chunks' own boundary count (a **product** case) | `U4 region … = false` on those planes, exit 1 |
| **a region's faces are the chunks' faces summed** | the per-chunk mesh in that loop (a **product** case) | `the region 8013 = false`, exit 1 |
| **the declared quickstart digest is the document on disk** | `gameplay.rs`'s `U4_QUICKSTART_DIGEST` | `the_documents_the_rows_cite_are_the_ones_on_the_disk` |

**Re-expressing §39's twelve found three real things, and they are the honest part of this table.** (i) Its
side-rule bar `a_flat_uniform_chunk_is_six_faces` **no longer exists** — the extent contract's own first test
replaced it — so the case is re-pointed at the test that now refuses a chunk with walls. (ii) Its
columns-the-world-owns subject could **not be proved at all** on these bytes: the contract had given that rule
a second owner and disabling either owner alone changed nothing observable, which is a duplication rather than a
guard — so this pass **removed the shadow count**, and the case now perturbs the contract's own refusal.
(iii) The footprint's floor interior by declaration is **invisible to the module's flat test** (that world holds
solid ground below the footprint's floor), so its case is a **product** case: the independent recount keeps the
declaration and disagrees with the sweep (`1 289` against `2 313`) when it is dropped. Three more cases needed
no more than a string: `solid_at` and `water_at` gained the footprint as an argument, and `derive_region`
repeats the level test, so one arm had to be named.

### The defect the cascade found by running, and the fix at its cause

The U10 guard `the_documents_the_rows_cite_are_the_ones_on_the_disk` **refused this pass's own document edit**:
`clean-room/specs/004-u4-mesh-stream-work/quickstart.md` is the document the tree's rows declare **by digest**,
and §40's dated note moved its bytes (`b2bad610…` → `ca00f1be…`), so `U4_QUICKSTART_DIGEST` in
`src/gameplay.rs` no longer described the document on disk. Found by **running** the suite, not by reading it.
The cause is one constant and it is fixed there — the declared digest moved to the document's own bytes, the
guard is named where it bit (`specs/007-u10-gameplay-tree/quickstart.md` carries §40's note, two passes in a
row), and the printed `U10 documents:` line moved with it. It is a **finding**, not a repair: read-in-place is
the rule the tree was built to make work, and it worked on this pass's own work.

### An incident, recorded rather than hidden: the check file was destroyed and rebuilt

**Mid-pass a `write_file` destroyed `clean-room/tests/u4_geometry.rs`** — this pass's own end-to-end check —
and it was rebuilt from the pass's own earlier read of the file. §40's account above did not say so until an
amendment audit asked it to (2026-10-13); this section is that admission, written where the pass's own numbers
are and in the form §34's incident set: what is establishable, what is not, and what was done about the part
that is.

**Why nothing could restore it.** The whole `clean-room/` tree is **untracked**: `git ls-files clean-room`
answers nothing, `git status --porcelain -- clean-room` answers `?? clean-room/`, there is one branch (`main`)
and no stash, and `git log --all -- clean-room` is empty. (This plan and `docs/baselines/` beside it are
untracked too; among the files this section moves, only `docs/LEDGER.md` is tracked.) So there was no git
path — no `checkout`, no `restore`, no reflog — from which the file could be brought back.

**What survives of the lost bytes.** One digest: §39 recorded `tests/u4_geometry.rs` as `575a1cb7…`
(`baselines/u4-faces-over-the-cells/digests.txt` and its `proof.txt`). No copy of §39's bytes is on this
machine — searched for — and §39's harness perturbs **only `src/mesh.rs`** in all twelve of its cases, so no
case can be re-pointed at a byte range inside the rebuilt file either. **The four copies that do exist are a
different file**: the `%TEMP%` trees `cr-baseline/tests/`, `cr-base2/tests/`, `u6audit/copy/tests/` and
`u6audit/copy2/tests/`, byte-identical to one another at `47333a99…` — **319 lines and two tests**, the
§37-era file, not §39's, and not the file the rebuild was made from.

**What is establishable about the rebuild.** That it is §39-era content rather than an older baseline: it
carries §39's third test,
`the_laws_own_world_meshes_through_the_cells_door_and_re_keys_when_its_water_moves`, which **exists in no
copy on disk**; and the count progression matches the record — the §38-era log runs `running 2 tests`, §39's
87 integration tests are §38's 86 plus one, and this pass's 88 are §39's 87 plus `u4_geometry`'s fourth test.
The file on disk is **737 lines**, digest `f28c2aff…`, and `--test u4_geometry` runs **four** tests, green.

**What is not establishable.** Byte-identity to the lost file — and, named because it is the one failure mode
with no witness at all, **a dropped assertion inside the rebuilt file**: a test that no longer asserts
something still passes, so neither the suite nor the harness can see that loss.

**The part that is closable, and was closed.** The two tests the four `%TEMP%` copies share with the file on
disk were diffed line by line, and **no assertion the Oct-11 file carries is missing from disk**. Every
difference is one of three intended things: the §38 model move (a rectangle's content changed from a column
span to a voxel face, `Quad::cells()` no longer exists, and a column count that was the chunk's became the
footprint's own `ring_columns()`); §39's faces-and-ledger accounting (the independent recount of the same
cells, and `ledger.merged_away == ledger.faces - ledger.quads`); and §40's ring (`columns_produced ==
ring_columns()`, `samples_read == chunk_columns()`, the halo assertions). Two checks moved house and both
were followed: the literal `ledger.vertices == 4 * quads` / `ledger.indices == 6 * quads` pair is asserted on
these bytes by the module's own `six_indices_per_quad_exactly` (`src/mesh.rs`, which checks the mesh's own
numbers *and* the ledger's) and again by the new region test's `vertices() == quads * 4`; and the old
`ledger.cells == Σ quad.cells()` cross-check is not restorable as written, because a rectangle no longer
counts column cells — its successor is `mesh.area() == ledger.faces` beside the recount.

**Owed to the user to decide, and deliberately not acted on here.** A tree whose `src` alone is **39 937**
lines, under no version control, loses a file to one tool failure with no recourse, by construction. Naming
that is this section's job; choosing between version control for `clean-room/` and a durable snapshot before
any write is the user's, and this amendment takes no capability with it.

**What this amendment moves.** Nothing in the build, and no printed line. On the final bytes: `cargo test
--offline` → **417 passed / 0 failed**, exit `0`; `cargo clippy --offline --all-targets -- -D warnings` →
exit `0`; `cargo run --offline` → exit `0`, **522** lines of stdout, byte-identical to `final-stdout.txt`
(`6c77b9ad…`). No source digest moves — `tests/u4_geometry.rs` is still `f28c2aff…` — and the only digest
that moves is the harness's own, because the disclosure is written into its header as well: `run-guards.sh`
`d72f2fa5…` → `e433654fd0f68172…`, a comment-only edit whose behaviour, proof and restored sources are
unchanged.

### Where the contract's own account lives

The account lives **with the mesh**: `src/mesh.rs`'s module header now states **four** properties the module
exists to make checkable (the faces are the world's own voxels', a kept mesh cannot drift, the work accounting
is real, and **the extent is declared rather than inherited from the world**), names the **three** doors and the
one merge, and no longer calls the stitch owed; `Footprint`'s own doc comment states the three consequences of
the contract, each of them measured in the run. `src/evidence.rs` gained the instrument lines the run prints
and **no prose**, which follows the crate's pattern; the one departure-shaped fact is named instead of followed
silently: **`evidence.rs` is 5 138 lines** (from §39's 4 803, **+335**, the sixth pass in a row to grow it) and
its split is overdue rather than merely due.

### The done conditions, with exit statuses

`cargo test --offline` → **417 passed / 0 failed**, exit **0** — **329** library tests (unchanged) and **88**
across the integration suites (§39's 87 plus `u4_geometry`'s fourth test). `cargo clippy --offline
--all-targets -- -D warnings` → exit **0**. `cargo run --offline` → exit **0**, **522** lines of stdout,
*"Clean room OK"*. This pass's guard harness → exit **0**, twenty-two cases bit by name, every touched file
byte-exact, the rebuilt binary reproducing the start bytes, and the dated capture matching them. `sha256sum -c`
over `docs/baselines/u4-the-extent-contract/digests.txt` (run from `clean-room/`) verifies the sources this
pass touched, and `digests-captures.txt` (run from the baseline directory) the captures and the harness's own
proof.

### The four graded dimensions, with the top gap in each

| dimension | grade | what carries it, and the top gap |
|---|---|---|
| **Spec** — the record's own obligations, read as checks | **7/10** | §39's top Correctness gap — *the seam is declared, not measured* — is closed on its own terms: the rule is one named contract (`Footprint`), the reading reads the ring, a chunk's geometry is a function of its own columns, and a two-chunk rectangle is stitched and measured against the chunks' own faces. FR-002's one-digest-per-`(state, chunk, level)` still holds, and a world with no column under the request is refused by name from the contract itself. **Top gap**: slice 4's **per-chunk memory figure** is still `unchecked` — the run says so in its owed line rather than estimating bytes from a struct. |
| **Design** — one owner per piece of state | **7/10** | one owner for the footprints and their spans, one for the ring, one for the faces, one for the merge, one `merge_into`, one ledger and one independent recount; the contract's refusal is the **one** place the world's reach is judged — this pass removed the shadow count rather than leaving two owners. **Top gap**: `derive_region` is a **composition** call, not an entry: the resident set's unit is a chunk and a level, so a stitched mesh is kept nowhere and a wider region needs a residency of its own. |
| **Correctness** — measured behaviour on the final bytes | **8/10** | the two doors are compared **face for face over one world** and agree byte for byte; the three-world extent line prints the deciding numbers for the audit's old disagreement; a region's faces are checked against the chunks' own (`8 013 = 8 013`) and its seam against both sides' own counts (`6 = 6`); every count is still checked against an independent recount; and twenty-two guards bite by name on the final bytes. **Top gap**: the stitch stops at **one rectangle** — a seam *between* two regions is still meshed twice, and water's own sides are still its ground's faces. |
| **Quality** — the tree's own bar: honest refusals, no stale figure | **7/10** | the run prints both doors' extents and says which is which; the owed line moves the stitch to the region's own residency rather than claiming more; the audit's disagreement is settled with the numbers rather than explained away; re-expressing §39's twelve guards found a renamed bar, a **duplicated** rule (now removed) and a guard its own fixture could not see (now a product case) — all three recorded rather than papered over; and the U10 digest guard that refused this pass's own document edit is recorded with its cause. **Top gap**: **`evidence.rs` is 5 138 lines** and this pass added 335 of them — the sixth pass in a row, and the split is overdue. |

### Owed, and named

1. **A stitched mesh wider than one rectangle** — a region with a residency entry of its own and a streamed
   policy over it; a seam between two regions is currently meshed twice.
2. **Water's own side faces** — this seam still emits a water column's **top** face and declares its sides its
   ground's faces.
3. **The per-chunk memory figure** (slice 4's second half), still `unchecked` and printed as such.
4. **The displaced (256×) surface** — a rectangle of a displaced surface is not planar and must be subdivided.
5. **The GPU pass** over the plan-view instrument, the resizable window, the Sun's azimuth, **U9's** cubic save
   and **U13's** `P8` gate — kept out of this pass by the brief.
6. **Exit (b) — the surface's remainder inside the voxel**, and the two framings named unexercised inside
   `CubicWorld::digest` (unchanged from §37–§39).
7. **`evidence.rs`'s split** — the per-system slices moved into the modules they measure; the module split of
   `src/mesh.rs` was kept **out** of this pass by the brief, and is owed.
8. **Untouched and owed, exactly as before**: U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score`
   calibration sets, the score instrument's rungs, and U2's axis vocabulary.

### The next buildable step

**The GPU pass over the plan-view instrument** — U1's owed item, checked against `presentation::shade` and the
instrument's own projection rather than against a picture whose shape was refused (§37's ruling 4). U4's own
next step is the **region's residency**: the stitch that lands here is one footprint over one rectangle, and
the resident set's unit is still a chunk and a level.
## 41. The window draws the world's own geometry — one owner for the drawn reading, the plan reading kept and named, and the mesh's rectangles reaching pixels (2026-10-13)

§40 landed the extent contract and the stitch across two chunks — one footprint, one sweep, one merge — and it
printed every face that work produced. U4's geometry had been measured four passes in a row (the reading's
`313` quads, the cells' door's `327`, the region's `1227`, the recount's faces) and **not one of those
rectangles had ever reached a pixel**: the window §36 built presents `surface::Surface::render`, which is one
`shade` call per **column**, a `ZOOM × ZOOM` block of pixels each, and **no depth**. So the record stood in
exactly the shape the brief names — the mesh's quads reached print and never pixels, the owed next step was
the **GPU pass**, and a shader written beside an unbuilt seam would be a second render path before the first
one existed. This pass builds the first honest seam on the CPU: the mesh's own vertices and indices under one
camera with depth. It is not an engine; it is one owner for the drawn path, and a settlement of what happens
to the flat plan view rather than two render paths standing unlabelled.

### The seam, and its one owner

`src/drawn.rs` is new, and it owns the whole drawn path:

| what | owner |
|---|---|
| a rectangle's four corners and its two triangles | `drawn::corners` and `drawn::INDICES` — the mesh's own `Quad` |
| the eye, the field of view and the near/far planes | `drawn::Camera`, whose `over_extent` stands off at `radius / sin(fov / 2)` so the extent's own sphere subtends the declared view |
| a vertex to the frame | `Camera::project` — one projection, and a point at or behind the near plane is not projected |
| a triangle to pixels | `drawn::raster` — barycentric coverage, `1/z` interpolated, and the depth test that keeps the **shallowest** surface |
| a rectangle's colour | one `presentation::shade` call per rectangle, at the rectangle's own normal |
| a linear colour to a byte | `presentation::encode` — **moved here from `surface.rs`**, so both readings put their colours through one transfer function |
| the frame's own figures | `drawn::Drawn` and `DrawnCounts` — the counts, the coverage, the depth extents, the channel range and the frame's own digest |

Nothing is re-implemented. The shading is the contract's one call and its refusal is passed through rather
than replaced by a plausible colour (`shade_refusal` names the rectangle it happened at). The geometry is
`surface::region_mesh` — **one** call, through the mesher's region door — so the window's loop and the headless
frame cannot draw two different meshes, which is what makes the headless agreement bar below mean anything.
And one constant is the frame: `DRAWN_SIZE`, so the drawn frame and the plan frame are the same `576 × 576`
client and one window can show either reading.

### The plan reading is kept, named, and it is the evidence run's own frame

The brief allows two settlements of the flat plan view: retire it, or keep it named as one reading with one
owner. This pass **keeps it and names it**:

* the **drawn reading** is `src/drawn.rs` — the mesh's own rectangles, one `shade` call each, a depth buffer per
  pixel — and it is what the **window presents**;
* the **plan reading** is `Surface::render` — one `shade` call per column, one `ZOOM × ZOOM` block each, no
  depth — and it is kept: it **is the evidence run's own frame**, the one the `U1 client:` lines measure.

`src/surface.rs`'s module header carries the table of both readings with their owners and their doors and says
it plainly: *"Neither reading is unlabelled, and no frame is drawn by two of them."* §37's ruling 4 put that
module on the record as a plan-view instrument; §41 keeps it, and the owed GPU pass is still checked against
its projection.

**Naming it in the product cost five lines, and the reason belongs on the record.** This pass's first green
state printed **two frame digests in one run** — `U1 client: … frame digest bbdf3b91…` (the plan reading) and
`U4 drawn: … frame digest 409933f7…` (the drawn reading) — and only the drawn one was labelled. A reader of
that stdout cannot tell which reading moved, which is the failure the brief names word for word. So the three
`U1 client:` lines and `[U1]`'s registry row and its `interface:` line were re-worded to say which reading
each one is — *"the **plan reading** — kept and named, and the evidence run's own frame (the window presents
the drawn reading, §41)"* — and the window's own line already named both. The run's movement between that
first green state and the final bytes is recorded below as its own dated step rather than folded into one.

### What the run prints, and what each number is of

Two lines are added (522 → 524):

```
U4 drawn: the drawn reading: 1 frame(s) of the world's own geometry — quads submitted 339, vertices 1356,
triangles 678 (indices 2034), clipped 0, degenerate 4 — the last frame drew 153010 of 331776 pixel(s) (46.1%),
depth 53.05..60.32 m, 30 luminance band(s), channels 0x47..0xf2; frame digest 409933f7…, mesh digest
ef9079ab…, mesh state digest 3eb284a3…; the region is 32 × 32 column(s) of the world's own 48 × 48; camera eye
(16.00, 31.92, 129.92) m looking at (16.00, 16.00, 70.50) m, 45° of view, radius 23.54 m, distance 61.52 m,
near 0.62 m, far 615.20 m
U4 drawn: the drawn reading's own bars: the loop's own counts are the mesh's own = true; every rectangle is
inside the camera's own frustum = true; the picture is not a flat fill = true; the frame's mesh state is the
law's own state = true
```

Every number there is measured, and they cascade against the window's own run: one frame is `339` quads,
`1356` vertices, `678` triangles and `2034` indices, and the window's declared `200` frames are `67 800`,
`271 200`, `135 600` and `406 800` — the same counts times the same 200, with `4` degenerate rectangles a
frame (`800`) and none clipped. The frame's digest is the drawn frame's own bytes; the mesh digest is the
geometry the frame came from; and the **mesh state digest is the law's own state** — `3eb284a3…`, the same
digest the plane's state carries on every other line of the run — so "the frame is of the world the law
stepped" is checkable rather than asserted.

The drawn extent is the world's own **first chunk** (`32 × 32` of the `48 × 48` columns the plan reading
reads) and that difference is printed with every frame rather than smoothed over: the mesher's unit is a
chunk, and a drawn extent wider than one chunk is the region's own residency, which is still owed.

### The window, exercised for real, and the artifact it wrote

`cargo run --offline -- window 200 docs/baselines/u4-the-drawn-reading/frame.ppm` on the final bytes: exit
**0**, **9** lines of stdout, and a frame at `576 × 576` client pixels where **66 355 200 of 66 355 200**
pixels read back through the platform's own `BI_RGB` DIB agreed with the frame the window was handed. Its
numbers: `200` of `200` frames presented at `30` fps; `200` ticks of the plane's own law (`2 079` of `2 304`
columns standing water, deepest `7 809 µm` at column `330`, settled at the declared horizon); six materials
with water the material `5`; the Sun at `35°` with tint `(1.000, 0.925, 0.733)`; the last frame drawn under
one camera with depth, its headless twin equal to it (same frame digest), and the drawn bars all `= true`.

The artifact is **checked rather than trusted**:

* `frame.ppm` is **995 343** bytes = `15 + 576 · 576 · 3`, so it is exactly a P6 of the declared size;
* its **non-black pixels are 153 010**, which is the coverage the run printed (`drew 153010 of 331776`), to the
  pixel — the frame's untouched background is the black that makes up the other `178 766`;
* the printed channel range `0x47..0xf2` is the **drawn** pixels' own range while the whole image's minimum is
  `0x00` (the background), so the printed range describes exactly what it claims to;
* two runs of the window mode — the recorded one and this pass's own re-run on the final bytes — are
  **byte-identical** in both the nine lines (modulo the path they are handed) and the frame.

**What this honestly is not.** The frame holds **8** distinct colours — seven drawn plus the background —
while the run prints **30** luminance bands over its `339` shaded rectangles. Both are true and they describe
different things: the bands are the rectangles' own shade values (the contract's), the colours are those
values after the transfer function, and many rectangles share a shade because the region is mostly one plane
under one sun. The picture is a picture of the geometry, the light and the eye; it is not a per-pixel gradient,
a texture, a sky or a Sun, and those are the **GPU pass's** — owed, and named as owed, not implied.

### The guards, seventeen, proved to bite by name

`docs/baselines/u4-the-drawn-reading/run-guards.sh` perturbs **each guard's own subject** and shows the bar
bite **by name**: eleven **test cases** (a source perturbed, the named test run, its own assertion the name),
four **product cases** (the four drawn bars live on their own printed line, so only the binary's own output can
show them flipping to `= false` **with a non-zero exit**) and one **window case** (the same, through the
binary's `window` mode, whose bars are its own exit status since this pass). Its own proof is `proof.txt`:
**17 cases, 36 fault lines, exit 0**, every touched file byte-exact by digest, and the rebuilt binary
reproducing the run's own start bytes byte for byte.

| case | subject | what bit |
|---|---|---|
| the rectangle's corners are its four vertices | `drawn.rs` :: `at(span_u, span_v),` | `a_rectangles_four_corners_are_the_vertices_and_its_two_triangles_the_indices` |
| a rectangle's normal is the way it looks | `drawn.rs` :: `if quad.facing >= 0 { 1.0 } else { -1.0 };` | `a_rectangles_normal_is_its_axis_and_the_way_it_looks` |
| the eye's distance is radius over sin(fov over 2) | `drawn.rs` :: `radius / (fov_deg / 2.0).to_radians().sin()` | `a_camera_stands_off_so_the_extents_own_sphere_subtends_its_own_field_of_view` |
| a field of view that is not one is refused | `drawn.rs` :: `if !(0.0 < fov_deg && fov_deg < 180.0) {` | `a_camera_refuses_an_angle_that_is_not_one_a_view_with_no_direction_and_no_extent` |
| a point at or behind the near plane is not projected | `drawn.rs` :: `if depth < self.near \|\| !depth.is_finite() {` | `the_projection_puts_the_target_at_the_frames_own_centre` |
| the transfer function's low end is the standard's own | `presentation.rs` :: `12.92 * lit` | `a_flat_colour_encodes_to_the_byte_the_screen_is_handed` |
| the depth is the perspective-correct one | `drawn.rs` :: `1.0 / (w0 / a.2 + w1 / b.2 + w2 / c.2)` | `every_drawn_pixel_is_the_nearest_rectangle_the_mesh_holds` (the **independent recount**) |
| the depth test is what keeps a pixel | `drawn.rs` :: `if z < depth[at] {` | `the_drawn_frame_is_the_meshes_own_rectangles_to_pixels` |
| the frame carries the mesh's own state | `drawn.rs` :: `mesh_state: mesh.key().state,` | `the_drawn_frame_is_the_meshes_own_rectangles_to_pixels` (`the frame's mesh is keyed by the state the law is in`) |
| the camera stands off along the view's own direction | `surface.rs` :: `drawn::Camera::over(&mesh, view().toward, …)` | `the_camera_stands_the_regions_own_extent_in_front_of_its_own_frustum` |
| the frame and the recount encode at one exposure | `surface.rs` :: `exposure: EXPOSURE,` | `every_drawn_pixel_is_the_nearest_rectangle_the_mesh_holds` |
| the drawn frame is the client's declared size | `drawn.rs` :: `pub const DRAWN_SIZE: u32 = 576;` | `the_presented_frame_reaches_the_window_s_own_pixels` |
| **the loop's own counts are the mesh's own** | `drawn.rs` :: `vertices += 1;` (a **product** case) | `U4 drawn: … the loop's own counts are the mesh's own = false`, exit 1 |
| **every rectangle is inside the camera's own frustum** | `drawn.rs` :: `pub const NEAR_FRACTION: f64 = 0.01;` (a **product** case) | `… every rectangle is inside the camera's own frustum = false`, exit 1 |
| **the picture is not a flat fill** | `drawn.rs` :: `encode(shaded, look.exposure)` (a **product** case) | `… the picture is not a flat fill = false`, exit 1 |
| **the frame's mesh state is the law's own state** | `drawn.rs` :: `mesh_state: mesh.key().state,` (a **product** case) | `… the frame's mesh state is the law's own state = false`, exit 1 |
| **the window's own bars are its own exit status** | the same subject, through the binary's `window` mode | the window's barred line flipped, exit 1 |

Two of these are worth naming as what they are. The **depth** case had to be proved through the independent
recount in `tests/u4_drawn.rs`, not through the frame's own numbers: dropping the `1/z` interpolation changes
which rectangle wins a pixel, and only a recount that walks the mesh independently sees the difference — the
frame's own counts do not move when a pixel changes hands. And the **window case** exists because the window's
bars are the window's own exit status: a bar that fails in a window is a session that presented a picture it
cannot stand behind, so the status is non-zero and the refusal names the bar.

### §40's harness, re-run on these bytes

§40's harness perturbs `mesh.rs`, `evidence.rs`, `systems.rs`, `world.rs`, `gameplay.rs`, `tests/u4_geometry.rs`
and `tests/u6_bridge.rs` — and two of those, `evidence.rs` and `systems.rs`, are files **this pass moved**, both
of them again by the naming change above. So the previous harness was re-run on the final bytes: **exit 0**,
**22 cases**, every case bit by name, every touched file byte-exact by digest, the rebuilt binary reproducing
the pass's own run byte for byte, and the declared-quickstart-digest guard still holding (`gameplay.rs` did not
move this pass, so `specs/004-u4-mesh-stream-work/quickstart.md` is still the document the rows declare by
digest — §40's own defect was **not** re-opened). Its log is quoted in this pass's `digests.txt`; §40's own
`proof.txt` is left exactly as §40 recorded it, because a harness's proof is of the bytes it ran on and §40's
account is dated.

### The cascade: every count, line count and document move

| what moved | from | to |
|---|---|---|
| the suite | **417 passed / 0 failed** (§40: 329 lib + 88 integration) | **426 passed / 0 failed** (334 lib + 92 integration) |
| `cargo run --offline` stdout | **522** lines, `6c77b9ad…` | **524** lines, `25eb01ea…` |
| the window's own stdout | **5** lines in §36's own capture (`u1-visible-client/window-stdout.txt`, `59163d9c…`) and **6** in §37's (`4c53a0e8…`), both left exactly as recorded; §38–§40 recorded no window capture of their own | **9** lines, `6b3d0de8…` |
| the window's frame | §36's **plan** frame (`u1-visible-client/frame.ppm`, `86daeaf3…`), kept dated | the **drawn** frame (`u4-the-drawn-reading/frame.ppm`, `41df93b6…`), `995 343` bytes, re-verified on the final bytes |
| the drawn frame's digest | — | `409933f7…` (both the headless frame and the window's own last frame) |
| the mesh state digest the frame came from | — | `3eb284a3…` (the law's own state) |
| `evidence.rs` | 4 803 lines (§39), 5 138 (§40) | **5 167** lines |
| the guard harness | — | `run-guards.sh` `408f20a2…`, `proof.txt` (17 cases, 36 fault lines) |

**The five added tests** are the five `drawn::tests::` guards (§40's 329 → 334 library tests); the four added
integration tests are `tests/u4_drawn.rs`'s (`tests/u1_surface.rs`'s own eight are unchanged in count — its
`the_presented_frame_reaches_the_window_s_own_pixels` now checks the **drawn** reading's frame). **The two
added lines** are the `U4 drawn:` pair; **the seven changed lines** are `[U4]`'s row and `interface:` (the drawn
reading named where §40 could only count it), `[U1]`'s row and `interface:`, and the three `U1 client:` lines.
Nothing was removed. §36's, §37's, §38's, §39's and §40's own captures stand dated and **unmodified**: §40's
`final-stdout.txt` is this pass's `pre-stdout.txt` **by digest** (`6c77b9ad…`), exactly as §39's was §40's, so
the movement is between the record's own two ends.

### The done conditions, with exit statuses

`cargo test --offline` → **426 passed / 0 failed**, exit **0** — **334** library tests and **92** across the
integration suites. `cargo clippy --offline --all-targets -- -D warnings` → exit **0**, no warnings.
`cargo run --offline` → exit **0**, **524** lines of stdout, *"Clean room OK"*. The binary's `window` mode,
run for real on the final bytes → exit **0**, **9** lines, the frame written and re-verified. This pass's guard
harness → exit **0**, seventeen cases bit by name, every touched file byte-exact, the rebuilt binary
reproducing the run's own start bytes. §40's harness re-run on these bytes → exit **0**, twenty-two cases.
`sha256sum -c` over `docs/baselines/u4-the-drawn-reading/digests.txt` (run from `clean-room/`) verifies the
sources this pass touched, and `digests-captures.txt` (run from the baseline directory) the captures, the
frame and the harness's own proof.

### The four graded dimensions, with the top gap in each

| dimension | grade | what carries it, and the top gap |
|---|---|---|
| **Spec** — the record's own obligations, read as checks | **7/10** | U1's T009 *"a visible frame drawn, never `nodraw`"* now has a frame drawn from **U4's own geometry** rather than from the columns beside it, and U4's slice 4 has a consumer for the rectangles it counts. **Top gap**: slice 4's **per-chunk memory figure** is still `unchecked` — the run says so rather than estimating bytes from a struct — and the *streamed* half of the slice is untouched by this seam: one chunk is drawn, and nothing about it is streamed. |
| **Design** — one owner per piece of state | **8/10** | one owner for the corners and indices, one for the camera and its refusals, one for the raster and its depth test, one for the shading (the contract's call, not a second one), one for the transfer function (moved, not copied), one for the frame's figures and one for the counts — and **one reading each**, named, with the plan view kept as the evidence run's own frame rather than left standing beside a second unlabelled path. **Top gap**: the drawn extent is the world's **first chunk**; a drawn extent wider than a chunk needs the region's own residency (§40's own top Design gap), and the camera is placed by one `over_extent` call rather than by a declared scene. |
| **Correctness** — measured behaviour on the final bytes | **8/10** | the counts cascade exactly (`339 × 200 = 67 800`, and the same for vertices, triangles, indices and degenerates); the **headless frame and the window's own last frame are one digest** (`409933f7…`); the frame **artifact** agrees with the printed coverage to the pixel (`153 010`) and with the declared size to the byte (`995 343 = 15 + 576·576·3`); the state digest the frame carries is the law's own; two window runs are byte-identical; seventeen guards bite by name and §40's twenty-two re-run green. **Top gap**: the picture is still **8 colours over 30 shade bands** — no per-pixel shading, no sky, no Sun disc — and the depth buffer is a per-pixel `f64` with no culling beyond the frustum test and the bounding box. |
| **Quality** — the tree's own bar: honest refusals, no stale figure | **7/10** | every drawn bar is printed with its own name and its own answer; a camera that is not one, a view with no direction and a box that is not a box are refused by name; a `shade` refusal is passed through with the rectangle it happened at; **the two-digests-one-label defect was found by running the product and fixed at its cause**, and it is recorded rather than smoothed; the "what this is not" note puts the 8 colours beside the 30 bands; and every count, line count and digest in this account is the measured one. **Top gap**: **`evidence.rs` is 5 167 lines** and this pass added 29 of them — the seventh pass in a row to grow it, and the split is overdue rather than merely due. |

### Owed, and named

1. **The GPU pass** — U1's owed item and T009's other half: a shader checked against `presentation::shade`,
   with per-pixel shading, a sky and the Sun's disc. It now has **two** CPU readings to be checked against (the
   plan reading's per-column frame and the drawn reading's depth-tested frame), which is the strongest footing
   it has had.
2. **A drawn extent wider than one chunk** — the region's own residency, so the stitch §40 landed reaches
   pixels, and **water's own side faces** with it.
3. **The per-chunk memory figure** (slice 4's second half), still `unchecked` and printed as such.
4. **The displaced (256×) surface** — a rectangle of a displaced surface is not planar and must be subdivided.
5. **Exit (b) — the surface's remainder inside the voxel**, and the two framings named unexercised inside
   `CubicWorld::digest` (unchanged from §37–§40).
6. **`evidence.rs`'s split** — the per-system slices moved into the modules they measure; `src/mesh.rs`'s
   module split was kept **out** of this pass by the brief and is owed.
7. **A resizable window and the Sun's azimuth** — both named on U1's row and unchanged.
8. **Untouched and owed, exactly as before**: U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score`
   calibration sets, the score instrument's rungs, and U2's axis vocabulary.

### The next buildable step

**The GPU pass over the two CPU readings** — U1's owed item, still checked against `presentation::shade`, and
now against a **depth-tested** frame as well, so a shader's pixels have an independent subject rather than a
promise. U4's own next step is unchanged by this pass and sharper for it: **the region's own residency**, so
the drawn mesh can be wider than one chunk — the seam the window draws is one footprint over one chunk today,
and the stitch that would widen it is already measured.
