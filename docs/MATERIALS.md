# MATERIALS — the material system, scoped end to end

> **What this is.** The exhaustive inventory of the material system: every artifact,
> every consumer, every gap, each with a **disposition** and a **locator**. It exists
> because the user's ruling (`LEDGER.md` **C-122**) refounded the material target — the
> starter materials are a fabrication, and the real library is *derived from the real
> world* with **base patterns at the subvoxel and 256 levels** — and a refoundation
> must leave nothing silently unscoped.
>
> **Authority.** The user's rulings are the decisions; this file is the map that asks
> them. The old trees and the design package are **reference — evidence of a shape,
> never authority** (the standing ruling, `HYPERGRILLING.md`). `GOD_AGENTS.md` is
> read-only reference.
>
> **Method.** Every artifact gets one disposition:
> **BUILT** (code + test) · **SETTLED** (user ruling on record) · **OWED** (named
> work, unblocked) · **BLOCKED** (needs content or a decision) · **REFERENCE**
> (shape to learn from, not admitted) · **REFUSED** (a licence or rule forbids it).
> Nothing is left blank and nothing is "TBD".

Sources read for this pass: `LEDGER.md` C-8/C-27/C-38/C-39/C-45/C-122; `APPEARANCE.md`;
`STATUS.md`; `TODO.md`; `MODS-MAP.md`; `SHAPERS.md` §2/§3.5; `crates/voxel-model/src/`
(`material.rs`, `matter/`, `shaper.rs`); `crates/voxel-appearance/src/` (`composition.rs`,
`openpbr.rs`, `npr.rs`, `detail.rs`, `response.rs`); `crates/voxel-shapers/src/column.rs`;
and the references: `design_reference/{src/materials/*.rs, tools/materials/*, tests/material_*.rs}`,
`design_reference/docs/GRILLING-C8/C9` (as cited by `SCHEMA.md`/`SOURCES.md`),
`docs/research/earth-source-table.md`, `docs/superpowers/specs/2026-09-24-world-generator-core-design.md`
(§4.1–4.2, §5), and `shploingus-game/{flow-specs/, blocks/B-203, packages/}`.

---

## 1. Authority and provenance

| # | Element | Disposition | Locator |
|---|---|---|---|
| 1.1 | The target: a **huge derived material library** from real-world materials, with subvoxel + 256 base patterns; a derivation of the icon-builder system | **SETTLED** (verbatim, C-122) | `LEDGER.md` C-122 |
| 1.2 | Starter materials (`column::materials`, eight ids) | **REFUSED as authority** — fabricated; placeholder only | C-122, `column.rs` module docs |
| 1.3 | One material read at three **levels** (`body` 0.0 / `edge` +0.10 / `deep` −0.18 lightness offsets) | **REFERENCE** | `design_reference/tools/materials/declare.py` `MATERIAL_LEVELS`; `src/materials/world.rs` `Level` |
| 1.4 | `family × hue × level` identity (10 families: 7 icon + water/organic/soil; 8 hue anchors incl. `natural`) | **REFERENCE** | `declare.py` `HUE_ANCHORS`; `world.rs` `Family`/`Hue` |
| 1.5 | Generated table + `emit` + gate + digest (one home; failing closed; every defect reported) | **REFERENCE** | `tools/materials/{SCHEMA.md,declare.py,emit.py,schema.py}`; `src/materials/generated.rs` (`SOURCE_DIGEST`, "never hand-edit") |
| 1.6 | Sourcing markers **[V]** read · **[D]** declared · **[NS]** not yet sourced | **REFERENCE** (recommend adoption, R1-Q7) | `tools/materials/SOURCES.md` |
| 1.7 | "A source row registering content that does not exist is worse than an uncited claim" | **REFERENCE** | `SOURCES.md` (quoting `GOD_AGENTS.md` §11.8) |
| 1.8 | Earth quantitative source table (S01–S26; EARTH-GEO/CLM/HYD/ATM/ECO rows; every row "P-A (Q804); human review required") | **REFERENCE** — research evidence only, *not runtime data* | `docs/research/earth-source-table.md`; world-generator spec §4.1 |
| 1.9 | Typed **source/admission layer**: source rows → admitted generator-value rows with stable IDs, units, tolerances, scaling class, lifecycle, digest | **REFERENCE** (the design package's contract) | world-generator spec §4.1 |
| 1.10 | Lifecycle states `reference/provisional/admitted/derived/deferred/retired` | **REFERENCE**; the clean tree carries `Lifecycle` in `voxel-model::matter` | spec §4.1; `voxel-model/src/matter/` |
| 1.11 | Per-channel colour needs **sample spectra**, not class ranges; no invented hue | **SETTLED** | C-39; `APPEARANCE.md` §5 |
| 1.12 | OpenPBR values **derive from the material's own physics; refuse the rest** | **SETTLED** | C-27; `APPEARANCE.md` §5 |
| 1.13 | The mods' feature materials are the consumption target ("prepare here for the materials we use in the mods features") | **SETTLED** (user, this round) | C-122 §122.1 |
| 1.14 | Two licences forbid derivation: `BigGlobe` (ARR/NC + no-AI), `lithostitched` (no licence) | **REFUSED** | `MODS-MAP.md` §1 |

## 2. Identity and structure — what is built in this tree

| # | Artifact | Disposition | Locator / test |
|---|---|---|---|
| 2.1 | `MaterialId` — zero reserved for empty, never admitted | **BUILT** | `voxel-model/src/material.rs` |
| 2.2 | `Component { id, ratio, density_kg_m3 }` — exact rationals; per-component mass share | **BUILT** | same; F3's per-component display rule |
| 2.3 | `Material { id, name, components, optical_class, reflectance }` | **BUILT** | same |
| 2.4 | `MaterialRegistry` — **empty by default and stays empty unless a row is admitted**; admit collects *every* defect in one round; an unadmitted id refuses by name | **BUILT** | `material.rs`; C-8 |
| 2.5 | `Ratio` — exact rational; `bulk_density = Σ(ratio · density)`; `ratio_sum` exactly 1 for an admitted row | **BUILT** | `matter/rational.rs`, `material.rs` |
| 2.6 | `OpticalClass { Opaque, Transmissive, Scattering }` — `Scattering` is a **medium**, deliberately not a per-voxel material | **BUILT** | `material.rs` |
| 2.7 | `Reflectance { r, g, b }` exact, over-unity refused; `broadband` for achromatic | **BUILT** | C-39 |
| 2.8 | `SourcedReflectance { value, source }` — provenance is a property of the type | **BUILT** | C-39; test both sides |
| 2.9 | `Material.reflectance: Option<_>` — `None` is representable and means "nobody has a spectrum yet" | **BUILT** | C-39 |
| 2.10 | Seven sourced albedos (water 0.06, ice 0.50, regolith 0.17, sedimentary 0.40, metamorphic 0.22, igneous 0.12, bedrock 0.18) | **SETTLED** (user approved; C-39 table) | `STATUS.md` §21–22 |
| 2.11 | The eight starter ids and their lithology mapping (`for_lithology` / `lithology_of`) | **PLACEHOLDER, fabricated** — content to be replaced by the library; mapping interface stays | C-122 |
| 2.12 | `voxel-appearance` tests and `VoxelComposition` read `voxel_shapers::column::materials` — the fabricated registry is **load-bearing across crates** | **OWED** — replacement is a cross-crate cut, not a file swap | `composition.rs`, `openpbr.rs` test imports |
| 2.13 | Material as an *item*: `MaterialId` resolves in `voxel-instrument`; mass = count × admitted density in exact rationals | **BUILT** | `STATUS.md` §40 |

## 3. The derivation pipeline — real world → material

| # | Element | Disposition | Locator |
|---|---|---|---|
| 3.1 | `SUBSTANCES` table (16 rows: stone/sand/clay/coal/iron_ore/timber/plant_fibre/…), per- row `unit` (Mass/Volume/Count/Gas), `density`, `unit_mass`, `rot`, closed `tags`, `source` (mined/grown/gathered/salvaged/made), `sink` (holding/atmosphere), `no_consumer` reason | **REFERENCE** — none of it exists in this tree | `declare.py`; `SCHEMA.md` |
| 3.2 | `PROCESSES` table (7 rows, the stone rung: gather timber/stone/fibre, riven haft, knap a core, twist cord, assemble hatchet) with `tier`, exact-gram balancing inputs/outputs, `mechanism {heat_c, hours, labour_hours, power_kw}`, `requires {tool, skill, structure}`, `waste` | **REFERENCE** | `declare.py`; `SCHEMA.md` worked chain |
| 3.3 | `TIERS` — the stone rung's two ages (`hands & stone`, `bound & composite`); "the content is unbounded; an age is rows" | **REFERENCE** | `declare.py`; Q66/Q69 as cited |
| 3.4 | Unit rule: per-substance natural units; **canonical grams/mL/units**; conversions exact or refused; a float in the table is a defect | **REFERENCE** (recommend verbatim, R1-Q7) | `SCHEMA.md`; `schema.rs::grams` returns `None` rather than rounding |
| 3.5 | Gate rules: balance (quoting both totals), no orphan substance without a reason, no `made` output from a no-input process (a gather may only take what the world holds), dimension mixing refused, unknown names refused | **REFERENCE** | `SCHEMA.md`; `schema.rs::verify` |
| 3.6 | Double-entry mass ledger; **total exactly zero**, nonzero means a one-sided movement | **REFERENCE** — this tree has `voxel-instrument` mass and `conservation::Ledger`, not this double-entry world account | `materials/ledger.rs`; `crates/voxel-model/src/conservation.rs` |
| 3.7 | Seed-derived geology: deposits on tiles, integer noise, the **site** is the unit of depletion, mass from a declared scale factor | **REFERENCE** — this tree's `geology` has strata/ores but no tile-depletion account | `materials/geology.rs`; `crates/voxel-shapers/src/geology.rs` |
| 3.8 | Surface patches (brush/stands, regrowth as a function of elapsed time not ticks, stripped patches stop, harvest monotone) | **REFERENCE** | `materials/surface.rs` |
| 3.9 | Sim effects from declared rates (build cost, upkeep, wear = weakest part, desirability, nuisance; conduction deferred with a consumer) | **REFERENCE** | `materials/effects.rs` |
| 3.10 | Reduction/chain: a wanted good reduced to what the world gives up, integer runs, declared rounding, dependencies first, a plan that cannot reach the ground **refused by name** | **REFERENCE** | `materials/chain.rs` |
| 3.11 | The appearance derivation — mechanisms + chroma ceilings (a family's colour arrives by a named physical mechanism; more chroma is a *different material*) | **REFERENCE** | `declare.py` `MECHANISM`; `materials/mod.rs::within_ceiling` |
| 3.12 | The icon builder (CAP-010) as the derivation's source: declarations → generation → mechanical validation → manifests → promotion readiness | **REFERENCE** | design package `docs/design/capabilities/CAP-010.json` |

## 4. Properties a material row carries — current state

| # | Property | Disposition | Locator |
|---|---|---|---|
| 4.1 | Bulk density (exact rational) | **BUILT** | `material.rs` |
| 4.2 | Reflectance (sourced broadband albedo, 7 rows) | **BUILT** | C-39 |
| 4.3 | Optical class | **BUILT** | `material.rs` |
| 4.4 | Unit mass (for `Count` substances) | **OWED** — reference shape exists (`SCHEMA.md` table) | `SCHEMA.md` |
| 4.5 | Rot / decay (condition → mass loss → atmosphere) | **OWED** — mechanism arrives with the couplings; zero would be a placeholder | `declare.py` `rot` |
| 4.6 | Acoustics (absorption/reverberation/attenuation) | **OWED** — first asked by `sound-physics-remastered`; the reference's ask is **acoustics beside reflectance** | `MODS-MAP.md` §4 |
| 4.7 | Footstep/material timbre as a first-class block property | **OWED** — FC-10.2's destination recorded; no field exists in `voxel-model` (measured: no `acoustic`/`footstep` matches under `crates/`) | `MODS-MAP.md` §4 |
| 4.8 | Thermal (phase change, fire/heat, calcination) | **BLOCKED** — needs `SOURCES.md` [NS]/[D] heat rows before a number may be used | `SOURCES.md`; earth-source table §C/§D |
| 4.9 | Base pattern at the **64 stored slots** | **OWED** (the library's core); the slot mechanism is built | C-122; `shaper.rs` `SlotWriter` |
| 4.10 | Base pattern at the **256 addressed level** | **OWED**; the lattice exists but currently varies roughness/reflectance by hash — it is **not yet a material's pattern** | `voxel-appearance/src/detail.rs`; `SHAPERS.md` §3.5 ("a `Flowstone` ripple is not yet a ripple") |
| 4.11 | Per-channel hue | **BUILT for six of seven** — whole USGS v7 spectra retained and the exact visible-window derivation landed in `voxel_model::materials::spectra`; ICE held (no visible channels in the only H2O-ice AREF; C-143); every new library row sources the same way | C-39; C-140/C-141; C-143; `APPEARANCE.md` §5 |
| 4.12 | **Full sampled spectrum** retained per row (the whole measured range, exact); the r/g/b render values are a **derived view**, not the stored datum | **BUILT** — committed, compiled, digest-gated (C-143) | C-141; C-143 |
| 4.13 | **Appearance derives from composition**: reflectance = exact rational weighted blend of constituent spectra through `Component { id, ratio }` | **SETTLED** (user question + squaring, C-141) | C-141; `material.rs` |

## 5. The runtime consumption path — material → world

| # | Artifact | Disposition | Locator / test |
|---|---|---|---|
| 5.1 | Field-plane material rules: `material_of` runs the column's exact test order (above ground: sea/air; below: soil, rock, void) through field queries | **BUILT** | `chunk_fields.rs`; `the_field_plane_materializes_exactly_what_the_column_profile_says` |
| 5.2 | `MaterialId` → mesher slot; greedy merge by material; `VoxelShape` per voxel | **BUILT** | `voxel-mesh` |
| 5.3 | `VoxelShape` composition → appearance (`VoxelComposition` of a face, blend over 64 subvoxels weighted by visible sub-face area) | **BUILT** | `composition.rs` |
| 5.4 | OpenPBR derivation from composition: roughness from density against the admitted range; transmission from `OpticalClass`; coat from hydrology adjacency; `base_metalness` 0; **base colour sourced/absent**; six parameters named absent | **BUILT** | `openpbr.rs`; C-39; `STATUS.md` §135–142 |
| 5.5 | NPR: Source-2 terms over the OpenPBR BSDF, tested on stated inputs | **BUILT** | `npr.rs`; `APPEARANCE.md` §1 |
| 5.6 | Shading-response measurement (band count per material; deadlock bar 1) | **BUILT** | `response.rs` |
| 5.7 | Two kinds × two levels in the shaper model: physical (occupancy) vs material (slot) shapers; a material shaper **cannot carve** by type | **BUILT** | `voxel-model/src/shaper.rs`; C-42–C-44 tests |
| 5.8 | 256-level `declare`: a Detail-claiming shaper that declares nothing is refused by name | **BUILT** | `ShaperSet::declare` |
| 5.9 | Shaper roster (Slab, LoopCut, Column family, Boulder, Flowstone, Coral) with sources | **BUILT as shapes** | `voxel-shapers/src/voxel.rs` |
| 5.10 | Shaper **role slots** (pillar inset, loop-cut seam, coral calyx) as destinations for the library's patterns; shape application into `shape_at`/`build_chunk` | **BLOCKED** — the material-library seam is unspecified (C-123); the vertical axis is **ruled: z up** (C-138), so the sky face is `PosZ`; the rest waits on materials steps 2–3 | C-122 §122.2, C-123 §123.3, C-138 |
| 5.11 | Mesher refinement (split at block boundaries, declared rate, refusal with the number in it) | **BUILT** with named limits: merge must stop at block boundaries for a displaced material; bare amplitude is not a ripple yet | C-44.5; `SHAPERS.md` §3.5 |
| 5.12 | The process layer's material coupling (geology/substrate/weathering/ecology) | **PARTIAL** — stages that consume materials exist as processes (C-121); no material property is read beyond lithology class | `process.rs` |
| 5.13 | Instrument mass coupling (`MaterialId` is an item) | **BUILT** | `voxel-instrument`; `STATUS.md` §40 |

## 6. Content — what the library must contain, and what blocks it

| # | Element | Disposition | Locator |
|---|---|---|---|
| 6.1 | The old project's state: **0 admitted materials, 0 recipes, 0 work rates, 0 yields** | **FINDING** | `shploingus-game/blocks/B-203` (`FEATURES.jsonl` event 0047) |
| 6.2 | The ore-to-blade rung: iron-ore grade by deposit, coke per tonne, blast temperatures, limestone/clinker rows, steelmaking, charcoal yield, timber/forage yields, bloomery heat | **BLOCKED [NS]** — "an invented ore grade would make the ledger look sourced while being invented" | `SOURCES.md` §"Not yet sourced" |
| 6.3 | Durations, labour, power — declared `[D]`, never cited | **REFERENCE** | `SOURCES.md` §"Declared" |
| 6.4 | Every Earth row provisional under **Q804**, human review required before release | **REFERENCE** — the admission warning is part of the evidence | `earth-source-table.md` |
| 6.5 | The library itself (the "huge" set) | **NOT STARTED** — identity, pattern and derivation shapes are identified; content is the work | C-122 |
| 6.6 | Mods feature materials (footsteps, rain-by-material, absorption/attenuation, dynamic lights, weathering/rust, dense ores, seasons, biomes) | **OWED** — each has a destination row and none is a per-mod port | `MODS-MAP.md` §3–§4 |
| 6.7 | The material **registry digest / generated-source check** ("a stale generated file is caught") | **REFERENCE** | `materials/mod.rs`; `tests/material_manifest.rs` |

## 7. What this system is allowed to say — the boundaries

| # | Boundary | Disposition |
|---|---|---|
| 7.1 | **Refuse, never default**: an unadmitted material is a refusal, not a grey cube | **SETTLED** (C-8, C-45) |
| 7.2 | No value invented to make something render (C-38 is the record of what that costs) | **SETTLED** |
| 7.3 | A reflectance/quantity is carried **with its source** or not at all | **SETTLED** (C-39) |
| 7.4 | No floats in declared material content; exact rationals in the ledger | **REFERENCE** (recommend adoption) |
| 7.5 | A class value inside a published range is **this project's choice**, stated as such | **SETTLED** (C-39) |
| 7.6 | Nothing derived from a refused licence | **SETTLED** (`MODS-MAP.md` §1) |

## 8. The frontier — decisions that are the user's

Every question below is a **material** decision (scope, authority, content, dependency,
or unresolved ambiguity), with a recommendation. They are put as **hypergrilling Round
18** (`HYPERGRILLING.md`) and nothing downstream is implemented before they are answered.

**Round discipline (the user's correction, 2026-10-03):** a round carries **at most
eight questions**, and this ledger's protocol is governed by `HYPERGRILLING.md` — the
`grilling` skill's format is subordinate to it. Answers are recorded in the user's own
words in `HYPERGRILLING.md`, and a round ends with an explicit **continue / one more
round / pause / stop** choice.

* **Q1 — Library home and generator.** The declaration table + generator + generated
  table live **in this tree**, generated by a **Rust** generator (D97 bans Python),
  checked in, digest-verified, with no runtime parse — not in the old trees, which are
  reference and never authority. **Recommend: yes.**
* **Q2 — Identity.** `family × hue × level` is an **appearance view** over a
  substance-first row, not the material's identity; conservation keeps the substance
  identity. **Recommend: two views of one row.**
* **Q3 — Levels and roles.** Adopt `body/edge/deep` as the only level vocabulary and
  resolve shaper roles through it (pillar inset → edge, seam → deep, calyx → edge); a
  role that needs a *different substance* is refused rather than invented.
  **Recommend: yes.**
* **Q4 — Patterns.** A material row carries a **pattern identity**; the 64 slots stay
  stored per voxel; the 256 level evaluates the pattern on demand (addressed, never
  stored); no dense per-material pattern storage. **Recommend: yes.**
* **Q5 — The fabricated eight.** Keep them as the temporary lithology mapping (the
  world stays runnable), with the C-122 warning visible and **no new material role
  taken from them**; the library replaces their content in place.
  **Recommend: keep temporarily, replace in place.**
* **Q6 — First rung and property roster.** Schema (units/balance/gate) first, then the
  stone rung's content (16 substances, 7 processes, 2 tiers); row schema v1 carries
  density, reflectance, optical class and unit mass, and declares acoustics/thermal/rot
  as **empty columns under Availability discipline**, never zero. **Recommend: yes.**
* **Q7 — Sourcing discipline.** Adopt `[V]/[D]/[NS]`, "no float in the table", and
  "a `no_consumer` row must state why" **verbatim**. **Recommend: yes.**
* **Q8 — Order of work.** (1) schema+gate+generated table → (2) appearance levels →
  (3) base patterns at both levels → (4) shaper roster wiring → (5) mods feature
  materials. **Recommend: this order**; the shaper wiring unblocks only after (2)+(3).

## 9. What this document does not claim

* It does not admit a single material value. Every number in the references is
  evidence; the user's rulings and the gates admit content.
* It does not license any mod's code; licence verdicts are per reference
  (`MODS-MAP.md` §6).
* It is a scoping, not a plan of record: `TODO.md` carries the plan, and the answers
  to §8 become its rows.
