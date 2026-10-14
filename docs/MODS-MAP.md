# MODS-MAP — every reference the mods list names

> **The map's own reference count: 106 references** — one row per reference in §1–§5, in the
> record's own order: **10 in §1, 42 in §2, 19 in §3 and 35 in §4**, with §5's eight folding
> into rows already counted. That is the figure U10's tree derives as its entry count, so
> SC-001's equality is met against **this document's own rows**; the file-level counts below are
> counts of *other* files and are printed beside it rather than soldered onto it.
>
> **The three files, counted on 2026-10-12 rather than restated.**
> - `references/shploingus_dirty/references.md` — the file this map was written from (in the
>   sibling `shploingus-game` tree): **91 URL lines**, 91 distinct slugs. The "91" this header
>   has always quoted is *that* file's count, and it is still correct for it.
> - `design/mod-dispositions.jsonl`, beside it — the prior work: **83 rows** (**40 `mapped`**,
>   **43 `open`**), which left **8 URLs with no row at all**. Those eight are §5, since given rows.
> - `docs/references.md` — **this repo's** copy: **97 URL lines**, 97 distinct slugs. It is the
>   dirty file **minus three and plus nine**: gone are `C2ME`, `CubicChunks` (both still rows here,
>   §1) and `MinecraftFabricArchipelagoMod` (the replaced URL, kept in §4 and no longer an entry);
>   added are `TerraFirmaCraft`, `Calculator`, `Minecraft-CameraOverhaul`, `archipelago` (the
>   datapack), `immersivethunder`, `sound-physics-remastered`, `mc-ar-mod`, and two links that are
>   **not mods** — the `VictorTaelin` gist and the `blackflow.co.uk` article on greedy/backtracking.
>
> **How the three counts reconcile, each step measured.** **94** of the copy's 97 URLs correspond to
> a row — the **88** URLs the dirty file also carries, plus the **6** the copy adds that this map
> gives rows (`TerraFirmaCraft`, `Calculator`, `Minecraft-CameraOverhaul`, `archipelago`,
> `immersivethunder`, `sound-physics-remastered`) — with **3** spelling variants folding onto one row
> each (`chunky` → `MOD-chunky`, `tidal` → `MOD-tidal`,
> `stockbridge(refinedstrotage%2Bcreate)` → `stockbridge`). The **12** rows the copy does not carry are
> `C2ME` and `CubicChunks` (dropped from the copy, kept here) and the **10 Tekkit mods** the dated
> 2026-10-03 ruling added to §2 — `IC2`, `Railcraft`, `ComputerCraft`, `ProjectRed`, `BluePower`,
> `EE 3`, `ForestryMC`, `MFR`, `LogisticsPipes`, `ironchest` (`BuildCraft` is in both lists and is
> one row). So **94 rows carry a URL in the copy, 12 carry none, and 94 + 12 = 106** — the map's own
> count, above.
>
> **The old "83 + 8 = 91" balance, answered rather than dropped.** This header used to say that
> *"83 + 8 = 91 still balances, so either three URLs were added without the count being updated, or
> three were replaced"*, and to decline to pick a case. The files answer it: **neither** — the two
> references files differ by those three removals and nine additions, and this map's rows outnumber
> both because of the dated Tekkit ruling. `pc` is the one disposition that genuinely changed rather
> than merely appearing: it was filed as *"purpose unclear from the slug"* and is **Particle Core**,
> read from its own README.
>
> **What no row covers, named rather than left silent.** `mc-ar-mod` — a mod URL in
> `docs/references.md` that no section of this map names, so §6 names it **owed** — and the two
> non-mod links above, which are outside this map's scope by their own nature rather than dropped
> from it.
>
> **How to read a row.** `mapped` = the record maps it to a feature. `open` = the
> record explicitly left it undecided. `refused` = a licence fact forbids deriving
> from it (the refusals never depended on any other ruling and stand alone).
> `class` = the clean-repo destination.
>
> **Licence discipline:** a verdict is **per reference**, taken from the reference
> inventory. No blanket rule exists, and no milestone may grant one.
>
> **2026-10-03 (user):** *"the refusal list shouldn't stand … we're not using those
> mods source codes at all, just the ideas that they have"* — every reference is
> **readable for ideas**, including the licence-less ones; the verdicts govern
> **derivation**, never reading. Nothing from a refused reference is copied or
> implemented; the refusal verdicts stand for derivation.
>
> **2026-10-12 (user) — three rows corrected.** `tt20` is a **performance mod** (a TPS fixer: it
> changes how ticks behave when TPS is low) and is not the network/tick-budget row it was filed
> under; the Archipelago reference is **world generation** (island/archipelago terrain), not a
> multiplayer/randomiser — and its row is **re-keyed below** to the datapack that reference actually points
> at; and `resquake` is **not "purpose unclear"** — it is the Squake
> movement model, and it and `Minecraft-CameraOverhaul` were added to the game's player controller
> (`docs/LEDGER.md` **C-150**). That pair is **not in the clean room**, so neither is mapped to a
> clean-room row; the three rows below carry the correction.
>
> **2026-10-12 (user) — the Archipelago row re-keyed to the datapack.** `docs/references.md` carries the
> Modrinth **datapack** `archipelago` (`modrinth.com/datapack/archipelago`), not the mod this row was keyed
> to. The row is re-keyed, and its verdict is re-taken from the datapack's own terms — which declare
> **CC-BY-NC-ND-4.0** (Attribution–NonCommercial–NoDerivatives), a licence fact that forbids deriving — so it
> is **`refused`**. `MinecraftFabricArchipelagoMod` is demoted to the replaced URL it is: the Fabric client
> that *interacts with* the Archipelago randomiser, a different artifact. Nothing else in the map moves.
>
> **2026-10-12 (user) — the three references left without a row are given one.** `TerraFirmaCraft` and
> `Calculator` (both `mapped`, both read from their own clones and licence files — EUPL-1.2 and MIT) get
> rows in §2 above, and `Minecraft-CameraOverhaul` (GPL-3.0, read from its own repository) gets an `open`
> row in §4 beside `resquake`, on `resquake`'s footing: adopted into the game's player controller, **not in
> the clean room**, so assigned no clean-room row. Every one of the three states its evidence in its own row.

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

`*` = **`open`** in the record — **nine** of these rows carry the mark; `optical`, `create-new-age`
and `EE 3` are `open` without it, each for the reason its own entry states — **twelve** §2 `open`
entries in all. *(Corrected 2026-10-12, user): this legend used to say "24 of them", a figure no
source reproduces — the marks number nine, and the twelve is the nine plus the three named here.*
An `open` mod is not a rejection: it means *the record assigned no feature*, and this repo treats it
as **content to be classified when its slot is built**, not as decided.

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

**2026-10-12 (user) — the two content references that had no row, given one.** `TerraFirmaCraft` and
`Calculator` are in `docs/references.md` and are cited all through `docs/MODS-MATERIALS.md` §8.1, but
they were never in the S-01 … S-24 slots above and this map never gave them a row. Each is read from its
own clone (`_harvest/clone-hashes.txt`) and its own licence file, and each is filed under the **first**
`docs/MODS-MATERIALS.md` §8.1 rule that cites it — the **smaller reading**, taken and said: the harvest's
measured passage states a whole processing chain for a reference while the rule library states rules one at a
time, and a feature is a rule.

| Reference | Disposition | What it gives this repo, and its evidence |
|---|---|---|
| `TerraFirmaCraft` | **mapped** — `smelting` | the record's **platinum reference for earth-matter**, `github.com/TerraFirmaCraft/TerraFirmaCraft` (clone `6e77733c`, `docs/MODS-MATERIALS.md` §2): **20 rocks** each with a real geological class, **32 ores** carrying a **grade** system, **27 metals** and the real carburisation steel chain, **9 alloy recipes with exact composition ranges** (bronze = copper 0.88–0.92 + tin 0.08–0.12), the forge ladder as a **real colour-temperature model** (`Heat.java`: WARMING 0–80 … BRILLIANT_WHITE 1500–1600), and the processing chain as data (21 recipe dirs). **Licence: EUPL-1.2** (the European Union Public Licence v1.2, `LICENSE.txt` in the clone) — a copyleft licence that permits derivation; nothing is copied. It declares the harvest's **furnace-firing / smelting** rule, the first §8.1 rule citing it (*"TFC `heating` (883; 930 °C sample)"*); its world generation, climate, food and fauna content is **not claimed** — no row. |
| `Calculator` | **mapped** — `smelting` | read from its own clone (`c796f1dc`) of `github.com/SonarSonic/Calculator` (README: *"Calculator by Ollie Lansdell"*): circuits (board/dirty/damaged), purified and enriched coal, coal dust, reinforced iron ingot, amethyst and tanzanite sizes. **Licence: MIT** (`LICENSE`, © 2018 Ollie Lansdell) — derivation permitted. The harvest cites it under four §8.1 rules — furnace firing / smelting (*ReinforcedFurnace*), extraction (*Extraction/PrecisionChamber*), mechanical assembly (its assembly tiers), and circuit making / rework (its chambers) — and it is filed under the **first** of them; its circuits stay cited on the electronics rows, which other references declare. |

## 3. Presentation, performance and appearance

The record left this whole class open, expecting **one batch ruling per class**.
This repo takes it, as its own decision, in `docs/APPEARANCE.md`:

| Mods | Class ruling in this repo |
|---|---|
| `sodium`, `sodiumextras`, `sodiumleafculling`, `AcceleratedRendering`, `Flywheel`, `ImmediatelyFast` | **one architecture**: instanced/indirect batch submission with per-chunk culling. Never a per-mod port. |
| `pc` (= **Particle Core**) | **same architecture as the row above, one domain over**: *"doesn't render particles you can't see … only about a 12th of the lightbox is visible at standard FOVs."* Frustum culling applied to a different draw class, plus by-type spawn reduction. Never a per-class culler. |
| `SodiumOptionsAPI` | a **library**, not a mod — the options API `reeses-sodium-options` is built on. Same destination as that row (per-option dirty-state tracking, already adopted for this repo's settings); listed so the URL is not unaccounted for. |
| `lithium`, `FerriteCore`, `ModernFix`, `C2ME` (perf half) | **one architecture**: allocation-free hot paths and bounded caches. |
| `tt20`* | **performance**: a TPS fixer — how ticks behave when TPS is low. **The network/tick reading is retracted (2026-10-12, user): it is a performance mod.** No rule assigned yet. |
| `krypton`, `Redirected`* | **network/tick budget**: not applicable to a single-player-first client; recorded as `N/A until multiplayer`. |
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
| `neruina`*, `mercuryium-mod`* | open | *purpose unclear from the slug*; **unresolved, not guessed**. (`pc` and `resquake` were in this row and are **not any more** — `pc` is Particle Core, resolved in §3 from its own README; `resquake` is the Squake movement model, corrected below on 2026-10-12.) |
| `resquake`* | open | **the Squake movement model** — added to the game's player controller, with `Minecraft-CameraOverhaul` (its camera effects), in `docs/LEDGER.md` **C-150**; **not in the clean room**, so assigned no row here. |
| `Minecraft-CameraOverhaul` | open | **camera tilting and roll for the player controller** — its own repository's description: *"A Minecraft mod that makes gameplay more satisfying through the use of various camera tilting"* (`Mirsario/Minecraft-CameraOverhaul`); its effects were added to the game's player controller with `resquake` (`docs/LEDGER.md` **C-150**, `camera_overhaul.rs`). **Licence: GPL-3.0** (`LICENSE.md` in its own repository) — copyleft, derivation permitted, nothing copied. **Not in the clean room**, so assigned no row here — the same footing as `resquake` above (2026-10-12, user). |
| `ReplayMod`* | open | replay/camera recording — candidate for the debug views |
| `archipelago` | **refused** | **world generation**: island/archipelago terrain — the reference `docs/references.md` carries (`modrinth.com/datapack/archipelago`) is the Modrinth **datapack** `archipelago`, whose own terms declare **CC-BY-NC-ND-4.0** (Attribution–NonCommercial–NoDerivatives): a licence fact forbids deriving, so **refused** — readable for ideas, nothing derived (2026-10-12, user). |
| ~~`MinecraftFabricArchipelagoMod`~~ | *replaced URL* | **not this reference:** `github.com/Deadlydiamond98/MinecraftFabricArchipelagoMod` is the Fabric client that *interacts with* the Archipelago multiworld randomiser (`ArchipelagoServerConnector`, `MultiworldTraps`; branches `1.20.1`, `1.20.1-structure-rando-experiemental`), and its URL is no longer in `docs/references.md`. Kept so the replaced URL is accounted for rather than silent. |

## 5. The 8 URLs that had no disposition row — one slot since re-keyed (2026-10-12)

Surfaced by this repo, classified for the first time. Three are refusals (two in §1 and
one in §4, re-keyed 2026-10-12), two are LOD references (§1), two are Dynamic Trees flora (§1), and
one (`Tidal`) is unreviewed.

```
distant-horizons    → voxel-stream LOD reference
voxy                → voxel-stream hierarchical LOD reference
DynamicTrees        → voxel-shapers::ecology
DynamicTreesPlus    → voxel-shapers::ecology
Tidal               → open (candidate hydrology/tides)
BigGlobe            → REFUSED (licence)
lithostitched       → REFUSED (no licence declared)
archipelago         → REFUSED (its own terms: CC-BY-NC-ND-4.0 — non-commercial, no derivatives); world generation (island/archipelago terrain). The old name in this slot, MinecraftFabricArchipelagoMod, was the randomiser client, not this reference
```

## 6. What this map does **not** claim

- It does not license any code. Licence verdicts are per reference and the
  refusals above are the only ones this repo asserts on its own evidence.
- It does not resolve the **43 open** rows by fiat. An `open` row is a **station
  marker**, and every one of them is named in §2–§4 so that its absence from the
  build is visible rather than silent.
- It does not yet rule **`mc-ar-mod`** (`github.com/mt1006/mc-ar-mod`) — the one mod
  URL in `docs/references.md` that no section of this map names, and the reason the
  copy's 97 URLs and this map's 106 references do not meet in a single subtraction.
  No licence has been read for it and no disposition is taken: its row is **owed**,
  and the tree derives no entry from it rather than inventing one.
