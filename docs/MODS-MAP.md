# MODS-MAP — every URL in the mods `references.md`

> **Source:** `references/shploingus_dirty/references.md`, **91 URLs**.
> **Prior work:** `design/mod-dispositions.jsonl` = 83 rows (40 `mapped`, 43 `open`),
> leaving **8 URLs with no row at all** — listed in §5 and classified here for the
> first time.
>
> **How to read a row.** `mapped` = the record maps it to a feature. `open` = the
> record explicitly left it undecided. `refused` = a licence fact forbids deriving
> from it (the two refusals never depended on any other ruling and stand alone).
> `class` = the clean-repo destination.
>
> **Licence discipline:** a verdict is **per reference**, taken from the reference
> inventory. No blanket rule exists, and no milestone may grant one.

---

## 1. World shape and streaming — the load-bearing class

This is the class the first target ("a fully playable basic world") directly needs.

| ID | URL | Record | What it gives this repo |
|---|---|---|---|
| `MOD-c2me` | github.com/K0-RR/C2ME | mapped | **chunk infrastructure**: parallel chunk generation, thread-safe world access |
| `MOD-cubicchunks` | github.com/OpenCubicChunks/CubicChunks | mapped | **vertical extent**: cubic sections instead of a fixed column band |
| `distant-horizons` | gitlab.com/distant-horizons-team/distant-horizons | *no row* | **LOD horizon**: ringed detail falloff → `voxel-stream::horizon` |
| `voxy` | github.com/MCRcortex/voxy | *no row* | **LOD horizon**: hierarchical voxel LOD → `voxel-stream::horizon` |
| `MOD-chunky` | github.com/pop4959/Chunky | mapped | **pre-generation**: offline world build (FC-5.4) |
| `DynamicTrees` | github.com/DynamicTreesTeam/DynamicTrees | *no row* | **flora placement**: growth-model trees → `voxel-shapers::ecology` |
| `DynamicTreesPlus` | github.com/DynamicTreesTeam/DynamicTreesPlus | *no row* | addon to the above; same destination |
| `MOD-tidal` | github.com/Icywhenhot/Tidal | *no row* | unreviewed; candidate for `voxel-shapers::hydrology` (tides). **Open.** |
| `BigGlobe` | github.com/Builderb0y/BigGlobe | **refused** | ARR + CC-BY-NC on earlier versions, explicit no-AI clause. Readable to understand a problem; **nothing derived may be implemented** |
| `lithostitched` | github.com/Apollounknowndev/lithostitched | **refused** | no licence declared (verified 2026-09-25) |

**Adaptation boundary (already ruled):** C2ME and CubicChunks are **MIT** and
admissible **as architectural references only** — *use the ideas; do not import
engine-specific code; do not assume their thread model is correct.* This repo's
streamer is a fresh implementation of the *ideas* (cubic sections, residency-aware
scheduling, ringed LOD) over its own `(seed, tick, period)` determinism.

## 2. Industrial and content (the S-01 … S-24 slots)

All twenty-four slots were **kept**; these mods are the credits. None is on the
critical path to a playable world, and all are **content scheduled after it**.

| Group | Mods |
|---|---|
| **Create core + addons** | `Create`, `Create-Diesel-Generators`, `CreateExtraGauges`*, `create-factory-controller`, `Create-The_Factory_Must_Grow`, `extra-disks`, `cable-tiers`*, `stockbridge`, `refined-fluid-substitution`*, `create-storage-neo-forge`, `refined-storage`, `Create-Threaded-Trains`*, `createsifter`, `create-more-vertical-additions`*, `optical`, `Create-Ez-Stock-Ticker`, `create-metalwork`, `Create-Mobile-Packages`, `create-new-age`, `Create-Pantographs-and-Wires`*, `create-pressure-gauges`*, `Repackaged`* |
| **Chemistry / metallurgy** | `Rubberworks`, `SteamPowered`, `GregTech-Modern`, `Immersive-Weathering` |
| **Tools and parts** | `TinkersConstruct`, `Step-Crafter` |
| **Automation primitives** | `BuildCraft`*, `effortless` |

`*` = **`open`** in the record (24 of them). An `open` mod is not a rejection: it
means *the record assigned no feature*, and this repo treats it as **content to be
classified when its slot is built**, not as decided.

## 3. Presentation, performance and appearance

The record left this whole class open, expecting **one batch ruling per class**.
This repo takes it, as its own decision, in `docs/APPEARANCE.md`:

| Mods | Class ruling in this repo |
|---|---|
| `sodium`, `sodiumextras`, `sodiumleafculling`, `AcceleratedRendering`, `Flywheel`, `ImmediatelyFast` | **one architecture**: instanced/indirect batch submission with per-chunk culling. Never a per-mod port. |
| `lithium`, `FerriteCore`, `ModernFix`, `C2ME` (perf half) | **one architecture**: allocation-free hot paths and bounded caches. |
| `krypton`, `Redirected`, `tt20`* | **network/tick budget**: not applicable to a single-player-first client; recorded as `N/A until multiplayer`. |
| `MixinTraceReforged` | **debug tooling**: a refusal must name itself (this repo's `Refusal` already does, by construction). |
| `SodiumDynamicLights` | mapped: dynamic block-emitted light (FC-8.6) → the appearance layer. |
| `shuttfup`, `Stfu`* | open; R32 pointed from `shuttfup` to `Stfu`. **Unresolved** — recorded, not guessed. |
| `Argon4W/AcceleratedRendering`, `AcceleratedRecoiling`* | open; `AcceleratedRecoiling` has no record hit. **Unresolved.** |

## 4. UI, audio, ecology, worldgen content

| Mod | Record | Destination |
|---|---|---|
| `Jade` | mapped | **read-the-world overlay**: one sensor concept, overlay is a *consumer* → `voxel-client::overlay` |
| `JustEnoughItems` | mapped | one index, both directions (S-18) → later |
| `searchables` | mapped | one plain-text index (FC-9.6) → later |
| `MouseTweaks` | mapped | bulk inventory gestures (FC-9.10) → later |
| `pick-up-notifier` | mapped | pickup merge-and-refresh (FC-9.11) → later |
| `reeses-sodium-options` | mapped | **per-option dirty-state tracking** — adopted for this repo's settings |
| `enhancedtooltips`, `Minecraft-Smooth-Scrolling`* | open | ride the UI/design round |
| `lazy-language-loader`* | open | *no i18n row exists yet*; recorded as an owed decision |
| `AmbientSounds` | mapped | scanned local-geometry soundscape (FC-10.1) |
| `Cool-Rain` | mapped | rain sound **by material** (FC-10.3) |
| `Presence-Footsteps` | mapped | **material as a first-class block property** driving footsteps (FC-10.2) ← adopted into `voxel-model` |
| `matmos-ambient-sound` | mapped | spatialised ambient emitters (FC-10.8) |
| `biomemusic`* | open | *rodio dynamic audio for all audio* — verbatim R33; **unscoped** |
| `countereds-smooth-steps`* | open | might feed FC-10.2 |
| `SereneSeasons` | mapped | seasonal clock, continuous analogue ramp (FC-10.4) → `voxel-shapers::climate` |
| `Immersive-Weathering` | mapped | slow weathering with named permanent repair (FC-10.5), rust (FC-10.6) |
| `Ecologics` | mapped | biome triple + variants (FC-5.6/5.7) → `voxel-shapers::ecology` |
| `boids-reforged`, `Boids-Birds` | mapped | flocking fauna (FC-5.5) → `voxel-shapers::ecology` |
| `DenseOres` | mapped | dense ore vein size/yield dials (FC-5.3) → `voxel-shapers::geology` |
| `effortless` | mapped | fourteen parametric primitives, pattern transformers, undo (FC-6.1–6.3) |
| `curious-mobs`*, `personality`*, `Ratatouille`*, `water-your-crops`*, `SeedDelight`* | open | ecology/food rows; **unresolved** |
| `consistent-block-drops`* | open | block drops; **unresolved** |
| `no-more-knockback-latency`* | open | combat feel; **unresolved** |
| `pc`*, `resquake`*, `neruina`*, `mercuryium-mod`* | open | *purpose unclear from the slug*; **unresolved, not guessed** |
| `ReplayMod`* | open | replay/camera recording — candidate for the debug views |
| `MinecraftFabricArchipelagoMod` | *no row* | multiplayer/randomiser; **out of scope for a single-player-first world** |

## 5. The 8 URLs that had no disposition row

Surfaced by this repo, classified for the first time. Two are refusals (§1), two
are LOD references (§1), two are Dynamic Trees flora (§1), one is multiplayer, and
one (`Tidal`) is unreviewed.

```
distant-horizons    → voxel-stream LOD reference
voxy                → voxel-stream hierarchical LOD reference
DynamicTrees        → voxel-shapers::ecology
DynamicTreesPlus    → voxel-shapers::ecology
Tidal               → open (candidate hydrology/tides)
BigGlobe            → REFUSED (licence)
lithostitched       → REFUSED (no licence declared)
MinecraftFabricArchipelagoMod → out of scope (multiplayer)
```

## 6. What this map does **not** claim

- It does not license any code. Licence verdicts are per reference and the two
  refusals above are the only ones this repo asserts on its own evidence.
- It does not resolve the **43 open** rows by fiat. An `open` row is a **station
  marker**, and every one of them is named in §2–§4 so that its absence from the
  build is visible rather than silent.
