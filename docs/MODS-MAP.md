# MODS-MAP — every URL in the mods `references.md`

> **Source:** `references/shploingus_dirty/references.md`, **91 URLs**.
> **Prior work:** `design/mod-dispositions.jsonl` = 83 rows (40 `mapped`, 43 `open`),
> leaving **8 URLs with no row at all** — listed in §5 and classified here for the
> first time.
>
> **Reconciled against `references.md` on 2026-09-30, and the arithmetic above no
> longer holds.** `references.md` was updated after this map was written — a
> hypergrilling pass on the shploingus game added access to some of these mod systems
> and appended new URLs. Matching all **91** current URLs against this file by slug
> leaves **three with no row anywhere**: `sound-physics-remastered`,
> `immersivethunder` and `SodiumOptionsAPI`. All three are now given rows (§3, §4).
>
> **What that discrepancy is not, stated rather than guessed.** 83 + 8 = 91 still
> balances, so either three URLs were *added* without the count being updated, or three
> were *replaced* and the rows for the ones they displaced are still here under their
> old names. Nothing in this repository distinguishes those two cases, so this map does
> not pick one. `pc` is the one disposition that genuinely changed rather than merely
> appearing: it was filed as *"purpose unclear from the slug"* and is **Particle Core**,
> read from its own README.
>
> **How to read a row.** `mapped` = the record maps it to a feature. `open` = the
> record explicitly left it undecided. `refused` = a licence fact forbids deriving
> from it (the two refusals never depended on any other ruling and stand alone).
> `class` = the clean-repo destination.
>
> **Licence discipline:** a verdict is **per reference**, taken from the reference
> inventory. No blanket rule exists, and no milestone may grant one.
>
> **2026-10-03 (user):** *"the refusal list shouldn't stand … we're not using those
> mods source codes at all, just the ideas that they have"* — every reference is
> **readable for ideas**, including the licence-less ones; the verdicts govern
> **derivation**, never reading. Nothing from a refused reference is copied or
> implemented; the two refusal verdicts stand for derivation.

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

**2026-10-03 — the Tekkit Classic set.** *"tekkit classic (specifically what yogscast played in
their original jaffa factory series) … an insane feature pull if you could get it"* — the MC 1.2.5
pack joins the content class: IC2 (no public source; the archived mirror's compiled **jar** is read
via the jar avenue), BuildCraft (MMPL 1.0.1 / MPL-2.0), Railcraft (custom licence: no modified
distribution), ComputerCraft (CCPL), ProjectRed (MIT; RedPower 2's successor), BluePower (GPLv3;
RedPower 2 clone), EE 3 (GPLv3/LGPL; EE 2's successor — **magic deferred** to a future
Deadlock-integrated model), ForestryMC (LGPL v3), MFR (**no licence file** — joins the
no-derivation set), LogisticsPipes (MMPL), ironchest (GPLv3). Reading for ideas per the 2026-10-03
ruling; **no code derived**. Inventory + per-mod harvest: `docs/MODS-MATERIALS.md` §6.
**`Create-The_Factory_Must_Grow` has an explicit no-AI clause — the highest-caution reference: not
cloned, not read; any use needs the user's explicit sign-off first.**

Two further rulings from the same pass: **no minecraft progression** — material roles follow real
properties (diamond does real diamond things; BuildCraft's gold/diamond gears are refused as
progression artifacts) — and the **double-cover rule**: the older mods reappear as their Create-addon
descendants, so the roster dedupes to one row per material with both citations kept.

## 3. Presentation, performance and appearance

The record left this whole class open, expecting **one batch ruling per class**.
This repo takes it, as its own decision, in `docs/APPEARANCE.md`:

| Mods | Class ruling in this repo |
|---|---|
| `sodium`, `sodiumextras`, `sodiumleafculling`, `AcceleratedRendering`, `Flywheel`, `ImmediatelyFast` | **one architecture**: instanced/indirect batch submission with per-chunk culling. Never a per-mod port. |
| `pc` (= **Particle Core**) | **same architecture as the row above, one domain over**: *"doesn't render particles you can't see … only about a 12th of the lightbox is visible at standard FOVs."* Frustum culling applied to a different draw class, plus by-type spawn reduction. Never a per-class culler. |
| `SodiumOptionsAPI` | a **library**, not a mod — the options API `reeses-sodium-options` is built on. Same destination as that row (per-option dirty-state tracking, already adopted for this repo's settings); listed so the URL is not unaccounted for. |
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
| `sound-physics-remastered`* | **no row** (added to `references.md` after this map was written) | **sound absorption and reverberation per material** — *"realistic sound attenuation, reverberation, and absorption through blocks."* The absorption term is a **material acoustic property**, which is FC-10.2's record again and is the first reference here that asks for acoustics beside reflectance rather than beside footstep timbre. Attenuation and reverb → FC-10.8. |
| `immersivethunder`* | **no row** (same) | **event audio scaled by distance from the strike** — *"realistic and dynamic thunder sounds based on the player's distance from the lightning strike."* The *shape* of it (a per-event source whose level falls with distance) is FC-10.8's spatialised emitter, and it is the closest reference in the whole list to the onset layer this repo is building. **But it names a dependency this world does not have: there is no lightning, no storm cell, and no electrical field anywhere in `voxel-shapers`.** Adopting the audio without the storm would be adopting a sound for an event that cannot occur. |
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
| `resquake`*, `neruina`*, `mercuryium-mod`* | open | *purpose unclear from the slug*; **unresolved, not guessed**. (`pc` was in this row and is **not any more** — it is Particle Core, resolved in §3 from its own README.) |
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
