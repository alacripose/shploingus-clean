# DESIGN-SPEC — the authority for shploingus-clean

> **What this is.** The fresh design spec for this repository, produced by
> hypergrilling the past grilling records (`feature-grill/FEATURES.jsonl`,
> `design/{mod,slot,spec-section,candidate,wg}-dispositions.jsonl`) against the
> mods `references.md`. Every feature below traces to a record; nothing here is
> invented. Where the record left something open, this document says **open**
> rather than filling it — the one rule of the parent project, inherited whole:
>
> **Refuse, never default.** A value that cannot be validated is refused and named.
>
> **Authority for this repo** is the user's four rulings of 2026-09-29:
> (1) a new sibling repository, clean code; (2) it *consumes the package API* while
> all game code is fresh and mod ideas are ported; (3) the first target is a
> **fully playable basic world with all terrain shapers** developed from their
> research papers; (4) **free agency** — every decision is mine except **how the
> game looks**, which follows OpenPBR + NPR with **4×4×4 subvoxels** and
> **256× sub-subvoxels** for block-material detail.

---

## 1. The world model

### 1.1 Voxel, subvoxel, detail

| Level | Side | Count per parent | What it is for |
|---|---|---|---|
| **Voxel** | 1 m | — | the world block; the unit of collision, ownership and the ledger |
| **Subvoxel** | 4 × 4 × 4 per voxel | **64** | *shape and composition*: which of the voxel's cells are solid, and of what material |
| **Detail** | **256** per subvoxel axis | **256³, never stored dense** | *appearance*: the material detail lattice, addressed by index and evaluated procedurally |

The 4×4×4 grid is **stored** (64 cells, cheap, and it is what makes a voxel a
shape rather than a cube). The 256× detail lattice is **addressed, not stored**:
it is a deterministic function `detail(material, subvoxel, i, j, k) -> MaterialSample`
evaluated on demand by the appearance layer. Storing it densely would be
16,777,216 samples per voxel; addressing it costs one hash per sample and buys
the same picture.

### 1.2 Chunk, region, ladder

- **Chunk** = 32³ voxels (settled first-target contract: 1 m voxels, 32³ chunks,
  signed 32-bit coordinates).
- **Regional field ladder** = **20 levels** (F4), so the top level is
  `32 · 2¹⁹ = 16,777,216` blocks. At 1 m per block that is **16,777 km**, which
  is far more than a world needs; the ruling stands as a safe upper bound.

  ⚠️ **The record's stated reason is wrong by a factor of 1000, and the correction
  is recorded rather than inherited** (see `LEDGER.md` C-10). F4 justifies the
  20-level ladder with *"32 · 524288 = 16,777,216 blocks, about 16.8 km"* and
  calls level 15's 1,048,576 blocks *"a 1.05 km top level"*. Both figures treat
  1,048,576 **metres** as 1.05 **kilometres**. The arithmetic is pinned in
  `voxel-shapers::fields` by a test that asserts 16,777 km and asserts the 16.8
  figure does *not* reproduce.
- The ladder carries **lithology, elevation and soil depth only**. Climate is a
  *process*, not a field, and is read from process state (F4).

### 1.3 Determinism

The world is a pure function of `(seed, tick, declared period)`. Slow systems are
**lazily advanced** — recomputed on demand — never cached stale, so two sensors
reading a moment apart cannot see different geology. Rebuild reproduces truth
with digests; worker count is a pure performance knob that never changes output.

---

## 2. The thirteen features (from the feature grill)

| # | Feature | State in the record | In this repo |
|---|---|---|---|
| **F1** | World truth and replay | done — replay inputs recorded, not derived; multi-rate systems lazily advanced; population ceiling refuses rather than drops | `voxel-model` (determinism, refusal), `shploingus` (replay) |
| **F2** | Source admission and material registry | **blocked — zero admitted materials** | `voxel-model::material` (materials as named sets of components in exact rational ratios summing to 1) |
| **F3** | Matter graph and layered voxels | blocked — same zero content | `voxel-model::voxel` (layered subvoxels; kind is an attribute of matter, not the matter) |
| **F4** | Regional fields | partial — 20-level ladder, identity bound *by effect not by kind* | `voxel-shapers::fields` |
| **F5** | Geology, terrain, substrate, deposits | **the largest evidence gap**; both intended refs refused on licence | `voxel-shapers::{tectonics,geology,terrain,substrate}` |
| **F6** | Fluid roster and hydrology | settled — fluid is matter with real volume and mass | `voxel-shapers::hydrology` |
| **F7** | Climate, atmosphere, weather, biomes | settled — climate is a process | `voxel-shapers::{climate,atmosphere,weather}` |
| **F8** | Bounded ecology | settled — the ceiling refuses | `voxel-shapers::ecology` |
| **F9** | Process scheduling and dynamics | settled | `voxel-shapers` (stage graph) |
| **F10** | Deltas, Earthworks, conservation | settled — a recipe that is a mass pump must **declare itself one** | `voxel-shapers::{delta,earthworks}` |
| **F11** | Actors, governed work, cuboid selection | settled | deferred to the API layer |
| **F12** | Deterministic workers and horizons | `partial`; **streaming is unwired, not broken** | `voxel-mesh`, `voxel-stream` |
| **F13** | Presentation adapters | in scope | `voxel-client`, `voxel-appearance` |

### 2.1 The exact/approximate boundary (F1, binding)

Six declared **approximate** families sit against a **must-remain-exact** set:

- approximate — temperature; pressure and flow; process rates and cadences for
  erosion and tectonics; weather, atmosphere and climate; ecology population
  counts and coupling; regional field interpolation;
- **exact** — mass, volume, component lineage, fluid volume and mass, lifecycle counters.

If a tolerance-bearing quantity ever feeds mass or volume, the audited-to-the-gram
claim silently weakens. This is a **test**, not a note: see `voxel-model::conservation`.

---

## 3. The slots (S-01 … S-24, all `keep` after judging)

The slot manifest is the *feature* roster of the industrial half of the game. All
twenty-four were kept; two carry amendments (S-02, S-16). The four **shaper tools**
that matter to a world generator are S-14 (press and saw, *as a shaper tool*),
S-16 (fourteen parametric primitives, pattern transformers, undo — **both API and
ENGINE**), S-24 (plough and roller) and S-22 (dense-ore three dials).

The remaining slots (S-01 extraction, S-02 rotation/electricity, S-03 instruments,
S-04 clutches, S-05 carry-matter, S-06 storage, S-07 crushing, S-08 extractors,
S-09 alloys, S-10 parts, S-11 arities, S-12 band controller, S-13 weighing, S-15
tank authority, S-17 read-the-world, S-18 index, S-19 gestures, S-20 hearing,
S-21 seasons, S-23 flocking) are **content**, scheduled after the world is
playable. They are recorded here so their absence is visible, not silent.

  ⚠️ **Three of those slots are already built, and this sentence is therefore
  understating what exists** — `S-03` (instruments), `S-12` (band controller) and
  `S-13` (weighing), plus `FC-2.16`'s gauge wall, in `crates/voxel-instrument`.
  The schedule above is kept; the correction is recorded rather than inherited
  (see `LEDGER.md` C-25), and the ⚠️ is the same shape as §1.2's.

### 3.1 The axis surface

The axis crates are one layer, and each row is a question the tree asks about itself.

| crate | the question it answers |
|---|---|
| `voxel-axis-ladder` | the nine ladders, their tokens and the bare-token rule, re-exported from the model's one vocabulary |
| `voxel-axis-coverage` | every ladder has a row, and every unit claims its source's characters exactly once |
| `voxel-axis-unit` | the addressable paragraph, its exact range and its deterministic id |
| `voxel-axis-claim` | the exact quote, the prose kind with its basis, and the support only derive computes |
| `voxel-axis-view` | the three-section document whose every line resolves to a claim |
| `voxel-axis-civilians` | the civilian gates: a named need, a budget, an estimate, planned material and a checked completion |
| `voxel-axis-being` | what a living thing is before it is a mob, a plant or a civilian |
| `voxel-axis-matrix` | what each segment is verified at, and the recipe that fixes it |
| `voxel-axis-verify` | one reading per segment, from evidence this tree already holds |
| `voxel-axis-artifact` | the package as a value: cited, laddered, pinned and reproducible |
| `voxel-axis-security` | the key the pin is checked against, held outside the checkout |
| `voxel-axis-mcp` | the authority an agent speaks through, and what a grant permits |
| `voxel-axis-balance` | how much of this tree is CPU and GPU code, and which public visual items reach no renderer |

## 4. The spec sections (SPEC-1 … SPEC-11)

All kept except **SPEC-10, retired by user ruling.** The two that bind this repo
directly:

- **SPEC-3** — *single canonical world authority.* One world, one truth. No
  render-side copy that can drift.
- **SPEC-5** — *four horizons, computed not copied.* The render horizon, the
  simulation horizon, the residency horizon and the field horizon are separate
  typed things and must never be mixed (see `voxel-stream`).

---

## 5. Scale contract

| Quantity | Value | Source |
|---|---|---|
| Voxel | 1 m | settled first-target contract |
| Chunk | 32³ voxels | settled |
| Coordinates | signed 32-bit | settled |
| First playable band | z = −16 … +47 | settled, **explicitly not the long-term boundary** |
| Field ladder | 20 levels | F4 ruling |
| Subvoxels | 4×4×4 | appearance ruling |
| Material detail | 256× | appearance ruling |

## 5.1 The axis folder — the crates that read the game

`crates/axis/` holds the axis layer, in its own folder the way the civilians are, and its crates are
the only ones that read the game rather than draw it. Each is registered in the census with its own
witness, and each has a patch that states why it exists.

| Crate | What it is for | Record |
|---|---|---|
| **voxel-axis-ladder** | the axis vocabulary the layers above share: **nine ladders, one declaration.** **`voxel-model` sits under every axis crate.** It re-exports the model's one `AxisKind`, its tokens, the coverage table and the bare-token rule, and it names the two ladders the model's vocabulary gained: `prose_kind` (eight labels with no order) and `establishment` (active · inactive · stale · unchecked · refused). The axis layer never depends on the model-bearing `voxel-von` or the MCP server; a classifier's answer is an origin letter, not a session | `PATCH-042` §The axis set |
| **voxel-axis-coverage** | coverage and the partition audit: **every ladder has a row, and every character has one owner.** A forged coverage row is refused by axis name; `tile` refuses a gap, an overlap, an empty span and a span past the end, each by position | `PATCH-042` §The scrubber |
| **voxel-axis-unit** | the addressable unit: a paragraph's deterministic `unit_<sha40>` id, its exact character range and the source slice it quotes. The tiling is checked before a unit exists, so a deleted paragraph is a named gap rather than a shorter document | `PATCH-042` §The scrubber |
| **voxel-axis-claim** | the claim: one exact quote per unit, a prose kind whose basis is `heuristic:<signals>` or `heuristic:default`, and a support level the private constructor computes. A one-character change to the quote is refused by claim id | `PATCH-042` §The scrubber |
| **voxel-axis-view** | the document: three sections — Asked for · Declares · Gates — where every line resolves to a claim's own quote and kind. A claim the document carries but has no line is refused by its id, so a dropped paragraph cannot render as a shorter document. An empty section prints `none`; a line with no claim is refused | `PATCH-042` §The scrubber |
| **voxel-axis-civilians** | the civilian gates over the model's ported systems: a selection cites its need and has an estimate within budget; a task has its material planned and cannot complete by declaration; a ladder reading's state — held, absent or unreadable — is owned by `LadderReading` and consumed by `voxel-package::decision::rule_for_civilian`; an arrangement round-trips and a refused write leaves it untouched | `PATCH-042` §The civilians layer |
| **voxel-axis-being** | the being substrate: one record, seven faces (civilian · pilot · god · mob · plant · incarnate · ghost), a mobility, a life stage, needs, a spiritual signature and a creation record. A tree and a ghost are the same substrate before they are anything else | `PATCH-043` |
| **voxel-axis-matrix** | the calibration matrix: one cell per `(segment, axis)` over the seven segments — shell · ui · game · world · pattern · axis · tooling — with the ladder reading, the five-star row that must have run, the bronze-to-platinum tier **computed** from the cell's own requirements, and the readable recipe line that hands a person or an agent the fix | `PATCH-042` §The calibration matrix |
| **voxel-axis-verify** | the walk and the report: **the one place a matrix reading is produced.** It reads the census and its missing-docs pins, the conformance files, the workspace's declared witnesses and the ladder coverage table, and emits one cell per segment with the source of every row in its basis — or the decline that names the sources which could not witness it, because a source that did not run is never a pass | `PATCH-042` §The scrubber |
| **voxel-axis-artifact** | the package as a value: **the five requirements the record states, as gates.** Plain text, one statement per line; every statement cited at its witness's `file:line` and carrying a ladder reading with a basis and a support the tree derives; a `sha256` pin over every byte above it with the signing seam's signature; and a reproduction claim that re-reads every named file and refuses drift, a missing file, or a machine with no tree to re-derive | `PATCH-042` §The purpose this serves |
| **voxel-axis-security** | the trust boundary's key: **the pin is checked by a key, not assumed.** This machine's key is held in the per-user data directory — a path inside the checkout is refused, because a key in the repository is in every clone — the signature is HMAC-SHA256 over the package's bytes or a build digest, and the four refusals keep an unknown algorithm, an unknown key, an absent signature and a moved byte apart instead of collapsing them into one | `PATCH-042` §The axis layers |
| **voxel-axis-mcp** | the authority an agent-facing surface speaks through: **the layer, not a second server.** It mounts the verified matrix and its package, hands out the package's line for a named ticket and cell, issues a ticket-scoped grant signed by the security layer, and **decides** an apply only when the credential is signed by a key it holds, names this ticket and this cell, has not expired, and the cell was measured with a witness — returning a receipt of the checks that ran and performing nothing | `PATCH-044` |

**The route an agent takes into the game.** `voxel-mcp` serves the authority as four tools, and each
is a row of the tree's own bridge rather than a second vocabulary:

| Tool | Answers | Bridge row (capability) |
|---|---|---|
| `get_axis_matrix` | the measured cells and the package they were written into | `read_governance` (observe) |
| `get_axis_recipe` | the package's line for a named ticket and cell | `read_recipe` (observe) |
| `grant_recipe` | a ticket-scoped grant, signed by this machine's key, as one wire line | `file_ticket` (present) |
| `apply_recipe` | the six-check decision under that line, and the receipt naming them | `resolve_ticket` (present) |

**The gate runs first and the signature runs second.** `apply_recipe` maps to `resolve_ticket`, whose
`RequiresGrant` class is what refuses a call with no credential — the bridge's own sentence, not this
surface's — and the axis layer is what checks the signature, the ticket, the cell, the expiry and the
witness. The expiry is measured against the server's clock, never a tick the client supplies. A
malformed credential is `malformed_record`, not a missing one, because telling a client "no grant was
presented" when one was would name the wrong fact.

**None of the six exists to hold state.** The being is a record a producer writes; the matrix is a
projection over cells; the verifier is the reader that builds them from the tree; the artifact is the
document those cells are written into; the security layer holds a key outside the tree, and the
authority layer is a view over the other five — and none of them writes anything back; a second store
beside the chain is refused by the security layer, and no crate here opens a file for writing.

## 5.2 Who owns the frame's geometry

Four owners, one direction, and each one is the only writer of what it holds:

| Owner | Holds | Writes when |
|---|---|---|
| `Streamer` (`voxel-work`) | the resident meshes, and **`residency_generation`** — the count of meshes entering, leaving or being replaced | a chunk arrives, is evicted, is demoted to stale, or the world is rebuilt wholesale. `world_generation` stays the *record's* number and moves only on an edit; a cache keyed on that alone keeps drawing a window the streamer has filled |
| `SceneCache` (`voxel-client::scene`, owned by `App`) | the frame's **terrain** scene — the world's rectangles, corners and origins — and the `SceneKey` it was built under (`world`, `residency`, `light`) | the key moves, **or** the build measured a light cell it had not measured before (coverage still filling in), **or** a caller refuses to keep it. `take` strips the previous frame's cloud and body suffix; `put` hands the terrain back |
| `WorldPipeline` (`voxel-client::world`) | the GPU buffers: quad records, origins, **vertices, indices** — and the scene build they were last written from | a write whose values differ, keyed on the scene's `build` id for the corners and on the record/origin arrays for the rectangles. Buffers grow by doubling and are never created per frame |
| `shading.wgsl` | the per-quad expansion (`vs_quad`, from a 24-byte record) and the per-corner placement (`vs_main`: `position + origins[chunk]`, where the origin is already `origin − eye`) | every vertex, every frame — which is the point: a corner is **chunk-local**, so walking does not rewrite it and no per-frame CPU work produces it |

The frame's path is therefore: pump → key → (build only if the key moved) → append cloud and bodies → draw → put back.
A frame that walks through a settled world does **no scene build, no corner upload and no allocation for either**, which is what
`geometry\t…` on the shipped binary's second-boundary line reports.

## 6. What is **open** in this repository

Recorded, never defaulted:

1. **Materials are zero.** F2/F3 are blocked on content that does not exist
   anywhere in the record. This repo ships a *registry and a conservation model*
   with a small, honestly-labelled starter set, and every consumer must treat an
   unadmitted material as a refusal.
2. **Worldgen Q's:** 6 of 91 rows are `retire`; they are honoured, not quietly kept.
3. **The rendering/perf mod class** (`sodium`, `lithium`, `ferritecore`,
   `immediatelyfast`, `modernfix`, `krypton`, `sodiumextras`, `flywheel`,
   `acceleratedrendering`, `mixintracereforged`, `sodiumleafculling`) has no
   per-mod ruling — it is a single architectural class in this repo: *how a
   renderer batches and culls*. See `docs/MODS-MAP.md` §3.
