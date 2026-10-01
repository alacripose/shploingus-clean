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
