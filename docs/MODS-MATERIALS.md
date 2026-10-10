# MODS-MATERIALS — the conclusive materials list read from the mods' source

> **Mandate (user, 2026-10-03):** *"You can get a conclusive list of the materials I want from the
> mods source code as long as we stay within the earth model and not the minecraft model."* — **the
> earth model, never the minecraft model.** *"also create does dumb shit with the belts made of kelp
> we want to avoid doing that"* — the mods' content is **evidence to study**; their invented
> mechanics are not adopted (per-feature reading, study never copy: H-24.8).
>
> **Where the sources are:** `F:/shploingus-mod-sources/` — shallow clones (`--depth 1`, no history),
> **43 trees** (original 14 + Tekkit 13 + Stage B 7 + Stage C 9) plus the Railcraft-API submodule, all
> HEAD-pinned in `_harvest/clone-hashes.txt`. A licence verdict is **per reference** (`docs/MODS-MAP.md` §1); the two refusals
> stand, and nothing here is copied into the tree.
>
> **Status: first pass.** Each row names the mod and the file it was read from. The later passes are
> listed in §3; nothing is claimed complete that is not measured below.

## 0. The model filter

| Keep (earth model) | Refuse (minecraft model) |
|---|---|
| elements, minerals, rocks, soils, metals, real alloys, fluids (water, oils, fuels, melts, gases), woods, fibres, polymers, ceramics, glass | substances with no real-world counterpart (ender, blaze, slime, nether, mana), invented mechanics (Create's kelp belts), and any value that exists only to make the mod's economy work |

A substance that exists in the real world **stays**, even when the mod invented its role. A substance
that exists only in the fictional world **goes**, even when it is mechanically convenient. A row that
cannot be classified yet is marked **?** rather than guessed.

**Mechanics filter (user, 2026-10-03):** *"we don't want create's model for everything … things
should not be powered by rotational energy, shafts should actually just be shafts, which is
something that a machine or belt would use"* — the mods are harvested for **materials and
processes**, never for a power model; Create's rotational/kinetic system (stress, rotation) is
not adopted, and a shaft is a physical part, not energy. The earlier asking this refers to is **found**
(2026-10-03): the original tree's modding-ledger-tech grilling — **0231 Q74** slot S-01 kinetic power
(Create's rotational set, candidates 1.1–1.21) and **0223 Q36** named source accounts per medium;
see `HYPERGRILLING.md` (the finding's addendum). The ruling revises S-01: no rotational power model,
shafts as physical parts.

## 1. Clone inventory (measured)

| mod | repo | measured |
|---|---|---|
| Create | Creators-of-Create/Create | 11,760 files; 2,017 Java |
| TinkersConstruct | SlimeKnights/TinkersConstruct | 447 tinkering-material JSONs |
| Create-Diesel-Generators | george8188625/Create-Diesel-Generators | fluids below |
| create-metalwork | AverageAnime/create-metalwork | molten fluids below |
| Rubberworks | hadron13/Rubberworks | resin below |
| GregTech-Modern | GregTechCEu/GregTech-Modern | **654** material registrations across 7 files |
| create-new-age | gitlab.com/antarcticgardens/create-new-age | cloned; scope pending |
| SteamPowered | git.tilera.org/Anvilcraft/SteamPowered | cloned; scope pending |
| createsifter | oierbravo/createsifter | cloned; scope pending |
| DenseOres | NewJumper/DenseOres | cloned; scope pending |
| optical | luccaPossamai/optical | cloned; **empty** (only `.git`) — nothing to harvest |
| TerraFirmaCraft | TerraFirmaCraft/TerraFirmaCraft | **43,394 files**; rocks/ores/metals below (added to `docs/references.md` by the user, line 93 — **the platinum reference**) |
| BigGlobe | Builderb0y/BigGlobe | idea source — licence-less; **read for ideas, no code used** (2026-10-03 ruling) |
| lithostitched | Apollounknowndev/lithostitched | idea source — no licence file; read for ideas, no code used |

**Revision pins (HEAD, 2026-10-03)** — full file: `F:/shploingus-mod-sources/_harvest/clone-hashes.txt`:

```
BigGlobe                  009c34aca9275678f00c78df90650e2f3f6ee423
Create                     a927008632b5eb9abc841d4c9c830995dc461038
Create-Diesel-Generators   bed7a1c4c52dc2249ca77cb8427ac3d250bb1588
DenseOres                  f354680079bc44d7953a5adde255991fe6e15e51
GregTech-Modern            0be6c8c079f40e1c79ce53edc9bc2624058f68ba
Rubberworks                b6c03e8afb76b703dc48a3ae563185ec23601c1e
SteamPowered               c3614a9885ad7ccfa113d226248d9260d243747a
TerraFirmaCraft            6e77733cba1e038653402205c65e306f17d0f901
TinkersConstruct           dce824f473c4515c4c1a1f8214838582fb69591a
create-metalwork           5a40940a83658b7f17cb1cf1e72649b8382e27bf
create-new-age             9ff732f553bad427ff0d7b0d8ba37213428fd057
createsifter               527e884ba61b8d9a521b242a58ebc1e4779f3c17
lithostitched              cd5482eeb09d8bc7a1db33a3fa1df057db40160b
optical                    8760ab0d7776a735abc95102ffaea8050d544436  (empty tree)
```

## 2. Harvest (first pass, by mod)

### Create
- **Metals** (`AllItems.java`, `foundation/data/recipe/CommonMetal.java`): iron, gold, copper, zinc,
  brass (Cu–Zn), aluminium, lead, nickel, osmium, platinum, quicksilver (mercury), silver, tin,
  uranium, constantan (Cu–Ni), electrum (Au–Ag), steel. All earth-real.
- **Stone types** (`content/decoration/palettes/AllPaletteStoneTypes.java`): limestone, scoria,
  scorchia — real volcanic/sedimentary rock names; asurine, crimsite, ochrum, veridium are invented
  (their real parent rocks are deepslate/calcite/tuff — the **parent** is real, the invented name is
  the mod's own): refuse the invented names, keep the real parents as the substances.
- **Fluids** (`AllFluids.java`): potion, tea, honey, chocolate. Potion/tea are invented — refuse;
  honey/chocolate are real foodstuffs — content question, not construction materials.
- **Process chains (measured 2026-10-03):** the full ore chain and all 20 process dirs extracted —
  `_harvest/create-processes.md` (digest) + `_harvest/create-processes/out.<process>` (unique result
  ids per process). Crushed-raw set (13 earth-real), washing → 9 nuggets + clay byproduct, kiln →
  ingot, mixing → brass; compat wrappers (IE, thermal, mekanism, ic2, silentgems …) recorded as
  cross-references, not roster rows.

### TinkersConstruct
- **Molten fluids** (measured names, `src/main/java`, `"molten_*"`): aluminium, brass, bronze,
  cadmium, chromium, cobalt, constantan, copper, electrum, gold, invar, iron, lead, nickel, osmium,
  pewter, pig_iron, platinum, silver, steel, tin, tungsten, zinc — real metals/alloys; plus real
  minerals (quartz, obsidian, glass, clay, porcelain, diamond, emerald) and fabricated melts
  (bendalloy, cinderslime, dawnstone, enderium, knightslime, lumium, manyullyn, nicrosil,
  queens_slime, refined_glowstone, signalum, slimesteel) — refuse the fabricated set.
- **Tinkering materials** (`src/generated/.../tinkering/materials/*.json`, 447 files): the real
  subset includes aluminum, andesite, bamboo, basalt, bone, bronze, calcite, clay, cobalt,
  constantan, copper, deepslate, diorite, electrum, flint, glass, gold, granite, hepatizon (real
  ancient bronze alloy — verify), ice, invar, iron, leather, lead, magnetite, obsidian, osmium,
  paper, pewter, pig_iron, platinum, quartz, rose_gold, silver, steel, stone, tin, tungsten, wood,
  wool, zinc. The rest are creatures/plants/fabrications — the filter decides per row.

### Create-Diesel-Generators
- **Fluids** (`CDGFluids.java`): plant_oil, crude_oil, biodiesel, diesel, gasoline, ethanol,
  concrete (16 dyed). All earth-real. **Refining chain measured** (`_harvest/stage-a-passes.md`):
  seeds → 100 mb plant_oil; fermentable + bone_meal → 400 mb ethanol; plant_oil+ethanol →
  biodiesel (1:1:2); crude oil distillation → diesel+gasoline (50/50 heated, 75/75 superheated);
  wood → chips; hammering → sheets; molds via cutting. Asphalt and 16 dyed concretes measured.

### create-metalwork
- **Molten fluids** — **pass done** (`_harvest/stage-a-passes.md`): casting layer over Create's mixer.
  Measured: brass = 9000 mb molten copper + 9000 mb molten zinc → 36000 mb molten brass; andesite
  alloy = 9000+1000 → 18000 mb; 9000 mb molten brass → 1 ingot. Own molten set: andesite, andesite
  alloy, brass, copper, gold, iron, zinc (+ netherite — refused). Real subset: brass, bronze, copper, electrum, gold,
  iron, manganese, osmium, palladium, platinum, silver, steel, tin, tungsten, zinc (aluminium,
  lead, nickel arrive via compat); fabricated — adamantite, aeternium, aquarium, banglum, calorite,
  carmot, celestium, cincinnasite, desh, durasteel, enderium, hallowed, kyber, kyrmot,
  metallurgium, midas_gold, morkite, mythantite, mythril, netherite, nikolite, orichadium,
  orichalcum, ostrum, prometheum, quadrillum, runite, star_platinum, starrite, stormyx, terminite,
  thallasium — refuse the fabricated set.

### Rubberworks
- **Resin** (`register/RubberworksFluids.java`; density 1500, viscosity 2000 [D]) — **items pass done**
  (`_harvest/stage-a-passes.md`): sapping log+leaves → 40 mb resin per species; 500 mb resin →
  2 rubber (heated); rubber → rubber_sheet (press); flower route 4 flowers + water → rubber
  (earth-plausible: real rubber also comes from flowering plants).

### GregTech-Modern — the big one
- **654 material registrations** across `common/data/materials/`: ElementMaterials (130),
  FirstDegreeMaterials (214), UnknownCompositionMaterials (132), OrganicChemistryMaterials (79),
  SecondDegreeMaterials (65), HigherDegreeMaterials (22), GCYMMaterials (12).
- **Composition is declared in the source**, e.g. andradite
  `.components(Calcium, 3, Iron, 2, Silicon, 3, Oxygen, 12)`, hematite `.components(Iron, 2,
  Oxygen, 3)`, asbestos `.components(Magnesium, 3, Silicon, 2, Hydrogen, 4, Oxygen, 9)` — the
  exact component ratios the library's `Component{ratio}` shape wants; elements carry
  `.element(...)`. Hazards are declared too (`asbestos → ASBESTOSIS`), which is exactly the
  acid–material-interaction ground Round 24 opened.
- **Extraction done (first cut):** `F:/shploingus-mod-sources/_harvest/gt-materials.tsv` —
  **520 unique material ids** carry a declared composition (130 elements + 390 compounds/materials),
  one row per material: `file \t id \t .components(...)|.element(...)`. The ~134 remaining of the 654
  registrations declare no composition (dyes, unknown-composition families) — a later pass.
- **Conditions read (first cut):** 143 `.temperature(...)` calls across the material files (fluid
  temperatures; sampled 1358 for copper-melt, 660, 1160, 1357, 300, 1542, 1285, 273 — **read the
  builder's unit before using any of them**), and the hazard vocabulary:
  `HazardTrigger.{ANY, INHALATION, SKIN_CONTACT}` × `GTMedicalConditions.{CHEMICAL_BURNS, POISON,
  WEAK_POISON, POISON with strengths, IRRITANT, NAUSEA, CARCINOGEN, ARSENICOSIS, ASBESTOSIS,
  CARBON_MONOXIDE_POISONING, METHANOL_POISONING}` — the acid/toxicity/interaction ground.
  `UnknownCompositionMaterials.java` holds **132** registrations (wood gas/vinegar/tar, charcoal
  byproducts, biomass, biodiesel, fermented biomass, creosote, diesel, rocket fuel, glue, …) whose
  composition is deliberately unspecified — the filter decides which are earth-model.

### TerraFirmaCraft (the platinum reference)
- **Rocks** (`common/blocks/rock/Rock.java`, 20, each with a **real geological class**):
  granite/diorite/gabbro (igneous intrusive), shale/claystone/limestone/conglomerate/dolomite/chert/
  chalk/tuff (sedimentary), rhyolite/basalt/andesite/dacite (extrusive), quartzite/slate/phyllite/
  schist/gneiss/marble (metamorphic). Each rock carries a full worked set: raw, hardened, smooth,
  cobble, bricks, gravel, spike, cracked/mossy bricks, chiseled, loose, pressure plate, button,
  aqueduct, rope anchor.
- **Ores** (`common/blocks/rock/Ore.java`): graded — native_copper, native_gold, hematite,
  native_silver, cassiterite, bismuthinite, garnierite, malachite, magnetite, limonite, sphalerite,
  tetrahedrite; normal — gypsum, cinnabar, cryolite, borax; powder — graphite, saltpeter, sulfur,
  sylvite; gems — amethyst, diamond, emerald, lapis_lazuli, opal, pyrite, ruby, sapphire, topaz;
  item-only — bituminous_coal, lignite, halite. All earth-real, with a **grade** system.
- **Metals** (`util/Metal.java`, 27): bismuth, bismuth_bronze, black_bronze, bronze, brass, copper,
  gold, nickel, rose_gold, silver, tin, zinc, sterling_silver, wrought_iron, cast_iron, pig_iron,
  steel, black_steel, blue_steel, red_steel, weak_steel, weak_blue_steel, weak_red_steel, high_carbon
  (steel/black/blue/red), unknown. The steel chain (weak → high carbon → steel → black/blue/red) is
  the real-world carburisation model the ore-to-blade rung is blocked on.
- **Alloy recipes carry exact composition ranges** (`generated/.../recipe/alloy/*.json`, 9): bronze =
  copper 0.88–0.92 + tin 0.08–0.12; plus bismuth_bronze, black_bronze, brass, rose_gold,
  sterling_silver, weak_steel/blue/red. Every bound is a decimal that converts to an exact rational
  (88/100 …) — the component-range shape the reference's ledger wants, read from a real metallurgy
  model (mark [D]: the *shape* is evidence, the final numbers still need their own sources).
- **The forge ladder is a real colour-temperature model** (`common/component/heat/Heat.java`):
  WARMING 0–80, HOT 80–210, VERY_HOT 210–480, FAINT_RED 480–580, DARK_RED 580–730, BRIGHT_RED
  730–930, ORANGE 930–1100, YELLOW 1100–1300, YELLOW_WHITE 1300–1400, WHITE 1400–1500,
  BRILLIANT_WHITE 1500–1600.
- **The processing chain is data** (`generated/.../recipe/` dirs): alloy, anvil, barrel, blast_furnace,
  bloomery, casting, chisel, collapse, crafting, glassworking, heating, knapping, landslide, loom,
  pot, quern, scraping, sewing, welding — the full earth-work vocabulary the stone rung grows into.

### SteamPowered
- Fluids (`SPFluids.java`): steam. Items (`SPItems.java`): bronze sheet, pressurized gas/steam
  containers.

### createsifter
- Meshes (`ModItems.java`): string, andesite, zinc, brass, sturdy, advanced brass/sturdy, custom.
- Recipes are datagen'd (`src/generated/resources/data/createsifter/…`) — **content pass done**
  (`_harvest/stage-a-passes.md`): pebbles (andesite/basalt/blackstone/diorite/granite/stone),
  crushed basalt/end stone/netherrack, the **crushed_raw_* set (13 metals)** + nuggets — the ore
  doubling by screening; seeds/saplings (farming), quartz, amethyst, coal, flint. Compat fantasy
  outputs (ae2/fluxnetworks/bigreactors) refused.

### DenseOres
- **Dense variants of the vanilla ores only** (`DenseBlockIds.java`): coal, iron, copper, gold,
  redstone, emerald, lapis, diamond (each plus its deepslate form), nether gold, nether quartz,
  ancient nether ore. Nothing new to the earth model; the *dense* mechanic is the mod's.

### create-new-age
- Multi-loader (`common/`, `fabric/`, `forge/`). The content tree is `electricity/ energising/ heat/
  motor/ nuclear/ ponders/` — **energy systems, not a material registry**. The earth-model yields are
  `corium` (the real meltdown product) and `nuclear/RadiationPoisoningEffect.java`; fluids are
  constants only (`BUCKET = 1000 / 81000`). No further material pass beyond corium.

## 3. Open (next passes)

1. **GT conditions pass** — temperatures, hazards, and the ~134 no-composition registrations; then
   the roster condensed into the library's families.
2. **create-new-age / createsifter / DenseOres** — content passes; **SteamPowered** is tiny;
   **optical** is empty (source needed if its content matters).
3. **TFC depth pass** — the smelting/anvil chain, the ore-grade system, and the rock-block recipes.
4. **Create process chains** — ~~crushing/washing/mixing/smelting outputs~~ **done 2026-10-03**
   (`_harvest/create-processes.md`; the content the stone rung grows into, measured).
5. **Mapping** into the reference's ten families and the `SUBSTANCES` table shape (H-19.2), with
   `[V]/[D]/[NS]` provenance per row.
6. **Evidence pinned (done 2026-10-03)** — all 14 clones carry their HEAD hashes in
   `F:/shploingus-mod-sources/_harvest/clone-hashes.txt`.

## 4. Reading any mod — the avenue (authorised 2026-10-03, corrected the same day)

The user's ruling: **the refusal list no longer bars reading.** *"the refusal list shouldn't stand,
they might have extra information we can use … we're not using those mods source codes at all, just
the ideas that they have"* — every reference is **readable for ideas**, including the licence-less
ones (`BigGlobe`, `lithostitched`) and the source-less ones whose **jar** is the avenue (assets and
data JSONs read directly with `unzip -l` / `unzip -p`; a class decompiled only when a question needs
its logic). What stays forbidden is unchanged and it is the only line: **no code is used, copied or
derived into this tree**; the per-reference licence verdict (`docs/MODS-MAP.md` §1) governs
*derivation*, never *reading*. An empty or gone repo is recorded as such rather than guessed around.
`optical` is the first jar candidate; the CurseForge-only mods (`sound-physics-remastered`,
`immersivethunder`) are candidates if their features need them.

## 5. The roster (first union, 2026-10-03)

The harvest's union so far, by earth category. `[V]` = read from the cited source; every numeric
value in a mod's source is **[D]** (a declared number — shape evidence, not a citation) until a
real-world source lands. The GT long tail lives in `_harvest/gt-material-ids.txt` (654 ids) and
`_harvest/gt-materials.tsv` (520 compositions).

**Metals & alloys.** TFC's 27 (bismuth … tungsten; the weak/high-carbon steel chain) · Create's 17
(iron, gold, copper, zinc, brass, aluminium, lead, nickel, osmium, platinum, quicksilver, silver,
tin, uranium, constantan, electrum, steel) · Tinkers' molten real subset · Metalwork's real molten
subset · GT's metals (aluminium, antimony, bismuth, cadmium, chromium, cobalt, copper, gold, iridium,
iron, lead, manganese, molybdenum, nickel, osmium, palladium, platinum, rhodium, silver, tantalum,
tin, titanium, tungsten, uranium, vanadium, zinc, and the alloys bronze/brass/steel/…) — long tail
in the TSV.

**Rocks, minerals, ores.** TFC's 20 classified rocks + 31 graded ores + gem set · Create's real
parents (limestone, scoria, scorchia) · GT's minerals (almandine, andradite, hematite, magnetite,
cassiterite, …) — long tail in the TSV.

**Fluids.** the water cycle (TFC/GT) · oils & fuels (Diesel-Generators: plant_oil, crude_oil,
biodiesel, diesel, gasoline, ethanol; GT: diesel, creosote, wood tar/vinegar) · melts (Tinkers and
Metalwork molten sets + TFC metal fluids) · steam (SteamPowered) · resin (Rubberworks) · gases
(GT air, ammonia, carbon monoxide …).

**Organics.** woods (TFC's species structures; Tinkers' treated/slime woods), fibres (Tinkers
string), hides/leather (Tinkers leather, TFC), rubber/resin (Rubberworks), foods (Create
honey/chocolate, TFC foods, forage) — a content question, not construction.

**Tekkit additions (2026-10-03).** Gems: ruby, sapphire, green sapphire, peridot, amethyst ·
metals/alloys: tin, silver, zinc, tungsten, brass, **tungsten carbide** · process materials:
**silicon boule/wafer (red/blue doped)**, flax, indigo, zinc plates · fluids: BuildCraft's
distillation roster (`_harvest/tekkit-fluids.tsv`), Forestry's bio-ethanol/biomass/seed oil/honey/
juice/milk (+ molten glass), MFR's biofuel/steam/milk/chocolate milk/mushroom soup/meat (dubious:
sludge, sewage) · Railcraft coke/creosote/slag. Refused: electrotine, teslatite, bluestone, coloured
alloys, UU matter, mob essence, pink slime, and the TFMG set (not read).

**Excluded so far (minecraft model).** Named fabrications: naquadah (GT's Stargate import),
ender/enderium, blaze, slime/slimesteel, queens_slime, mythril, the adamantite-style sets
(Metalwork), Create's invented stone names (their real parents kept), kelp-belt mechanics, and
Create's rotational power model (separate ruling).

**Next:** per-row family assignment (the ten families) + `[V]/[D]/[NS]` provenance, on the way to
the `SUBSTANCES` table shape.

## 6. The Tekkit Classic pull (Jaffa Factory) — added 2026-10-03

*"as well, tekkit classic (specifically what yogscast played in their original jaffa factory series)
that would be an insane feature pull if you could get it"*

Tekkit Classic = the MC 1.2.5 pack. Its surviving sources and successors were read under the standing
filters (earth model; **materials and processes only**; no power model — IC2's EU, BuildCraft's MJ
and RedPower's blutricity stay machine ideas). Thirteen trees were added on 2026-10-03 (27 in total
now), HEAD-pinned in `_harvest/clone-hashes.txt`.

| mod | repo | HEAD | licence (read from the clone) |
|---|---|---|---|
| IC2 — source closed, **jar route** | `mcdh/industrialcraft-2-experimental` (archived) | `fa3971b3` | mirror packaging **MIT**; the jar inside is IC2's binary (no public source; the official `IndustrialCraftMC/IndustrialCraft2` repo is a release container, 9 files) |
| IC2 Classic — API only | `TinyModularThings/IC2Classic` | `2d26c20b` | no licence file at repo root (README + gradle + `src/main/java/ic2/api`) |
| BuildCraft | `BuildCraft/BuildCraft` | `7c86626d` | MMPL 1.0.1 + MPL-2.0 (`LICENSE`, `LICENSE-NEW`) |
| Railcraft | `Railcraft/Railcraft` | `34416755` | *Railcraft Mod License* — no modified distribution; API MIT |
| ComputerCraft | `dan200/ComputerCraft` | `bbe7a4c1` | ComputerCraft Public License (MMPL-based) |
| ProjectRed — RedPower 2 successor | `MrTJP/ProjectRed` | `e631f983` | **MIT** |
| BluePower — RedPower 2 clone | `Qmunity/BluePower` | `65fd3e17` | **GPLv3** |
| EE 3 — EE 2 successor, magic | `pahimar/Equivalent-Exchange-3` | `1e2a2cbe` | GPLv3 + LGPL |
| Forestry | `ForestryMC/ForestryMC` | `72e15b0f` | **LGPL v3** |
| MFR | `skyboy/MineFactoryReloaded` | `15bfedb8` | **no licence file anywhere in the tree** |
| Logistics Pipes | `RS485/LogisticsPipes` | `122ea1c9` | MMPL 1.0.1 |
| Iron Chests | `progwml6/ironchest` | `bed1ebb0` | GPLv3 |

Not found / not cloned: **RedPower 2** (closed — ProjectRed/BluePower read instead), **EE 2**
(closed — EE 3 read; **magic deferred** to a future Deadlock-integrated model per
`HYPERGRILLING.md`), **Mystcraft** (no repo found; worldgen-flavoured, jar route if a feature needs
it), **Wireless Redstone** (no repo found; redstone logic, no materials),
**`Create-The_Factory_Must_Grow`** (explicit **no-AI clause** — **not cloned and not read**;
highest-caution reference).

**Harvest (first pass, measured).**

- **IC2 — the jar avenue works.** `IC2-mirror/industrialcraft-2-2.2.828a-experimental.jar` carries
the machine tables as **readable data** — `assets/ic2/config/{macerator, ore_washer, compressor,
extractor, blast_furnace, block_cutter, metal_former_cutting, metal_former_extruding, general}.ini`
— the fluid set as textures (biogas, biomass, constructionfoam, coolant, distilledwater, hotcoolant,
hotwater, pahoehoelava, steam, superheatedsteam, uumatter) and the item names in
`ic2/lang/en_US.properties`. Measured: the ore washer consumes **1000 mb water** per crushed ore
(`@fluid:1000`) → purified crushed ore + tiny dust + stone dust; the blast furnace takes an iron
ingot → refined iron + **`itemSlag`**; maceration covers lead/copper/tin/uranium/silver/gold/iron.
Names read: ingots (copper, tin, bronze, lead, silver, refined iron; *mixed metal — fabricated,
refuse*), dusts (stone, coal, iron, gold, copper, tin, lead, obsidian, lapis, **sulfur**,
**silicon dioxide**, diamond; *energium — fabricated, refuse*), plates + dense plates (bronze,
steel, copper, gold, iron, tin, lead, obsidian, lapis), casings. Pahoehoe, steam, superheated steam
and distilled water are earth-real; construction foam is process content; **UU matter is
fabricated — refuse**. Extraction done: `_harvest/ic2/` — `lang-en_US.properties` (1,022 lines)
→ 186 material keys in `lang-materials.txt`; **14 config tables** including `thermal_centrifuge.ini`,
`metal_former_rolling.ini`, `shaped_recipes.ini`, `shapeless_recipes.ini`, `uu_scan_values.ini`;
18 fluid textures in `fluid-textures.txt`.
- **BuildCraft** (`common/buildcraft/energy/BCEnergyFluids.java`): a real **oil distillation model**
— crude oil → residue / heavy / dense / distilled oils → dense / mixed-heavy / light / mixed-light /
gaseous fuels, plus **tar** — with declared density/viscosity/boil-point/spread/sticky/flammable
columns; extracted to `_harvest/tekkit-fluids.tsv`. Gears: wood, stone, iron, *gold, diamond*
(`core/BCCore.java`) — **gold and diamond refused as mechanics**: minecraft-progression artifacts
under the 2026-10-03 ruling (gold is soft; diamond does real diamond things). Silicon: redstone
chipsets — the redstone is minecraft-model (refuse); the "chipset" stays a process idea only.
- **Railcraft — values measured 2026-10-03** (`_harvest/railcraft-values.tsv`; the crafter constants live
  in the API submodule, cloned at its exact pin `3e334cd4`). Coke oven `DEFAULT_COOK_TIME=1800`: coal →
  coke + **500 ml creosote** (`ItemCoke.java`: `COKE_COOK_CREOSOTE=500`, coke heat 3200), coal block →
  coke block + 4500 ml, logs → charcoal + 250 ml (`ModuleFactory.java:392`). Blast furnace
  `SMELT_TIME=1280` per iron-value unit: iron ingot → steel + **slag 1**, block → block (slag 9),
  steel recycling at half time, item value units (bucket 3, helmet 5, chestplate 8 … door 6, trapdoor 6),
  fuels coke/charcoal (`BlockCoke`/`ModuleFactory`). Rock crusher `PROCESS_TIME=100`: obsidian →
  crushed + dust, coals/ores → dusts (sulfur 5+dusts, saltpeter 3+dusts), IC2 crushed ores ×2, Forestry
  apatite + sulfur/saltpeter by-product. Rolling machine `DEFAULT_PROCESS_TIME=100`: rails from iron 8 /
  bronze 8 / invar 12 / steel 16 / tungsten 32 / titanium 32 / tungstensteel 48. Coke, creosote and slag
  are earth-real; the trace gold-nugget/diamond crusher outputs are **minecraft-progression roles —
  refused** (C-127).
- **ProjectRed / BluePower (RedPower 2's descendants)**: real — rubies, sapphires, peridots (gems),
basalt, marble; ProjectRed Exploration adds raw tin/tin ingot, raw silver/silver ingot. BluePower
adds amethyst, green sapphire, silver/zinc/tungsten/brass dusts and ingots, copper, **tungsten
carbide**, **silicon boule → wafer → red/blue doped wafers**, **flax seeds/crop**, indigo dye, zinc
plates. Fabricated — **electrotine**, **teslatite** ("infused teslatite"), bluestone, blue/red/
purple alloy ingots (refuse). Keep the **alloy furnace** process (`IAlloyFurnaceRecipe`,
`AlloyFurnaceRegistry`).
- **Forestry**: biomass → **bio-ethanol** still (`ModuleFactory.STILL_DESTILLATION_*`; generator fuel
in `ModuleFluids`), bog earth / **peat** (`BlockBogEarth`), honey/wax/seed-oil vocabulary
(`EnumHoneyDrop`, `EnumHoneyComb`), apiculture + arboriculture — the family of the **Sjin's Farm
lead** (long-term deferral, `HYPERGRILLING.md`). `core/fluids/Fluids.java` constants with declared
densities **[D]**: BIO_ETHANOL 790, BIOMASS 400, **GLASS (molten) 2400**, FOR_HONEY 1420, JUICE,
MILK 1030, SEED_OIL 885.
- **MFR**: rubber wood/leaves/sapling, factory plastic, plastic pipes, ceramic dye
(`BlockRubberWood`, `BlockFactoryPlastic`, `ItemPlasticBoots`, `ItemCeramicDye`); `setup/MFRFluids.java`
densities **[D]**: milk 1050, biofuel 800, meat 2000, chocolate milk 1100, mushroom soup 1500,
**steam**; sludge 1700 and sewage 1200 are **dubious** (real wastewater analogues — decide in the
roster pass); **mob essence 400 and pink slime 3000 fabricated — refuse**. **No licence file** —
read-only ideas, no derivation, on the BigGlobe/lithostitched footing.
- **ComputerCraft / Logistics Pipes / Iron Chests**: no new earth materials (programmable computers;
logistics routing; chest tiers over the same ingots). Recorded so the pull is complete.
- **EE 3**: minium dust/shard/stone only — **magic, deferred** to the Deadlock-integrated model.

**Jaffa Factory specifics.** `grep -i jaffa` over every Tekkit clone finds **nothing** — the series'
item is not in these repos. The route, if a feature needs it, is the pack's own jars (Technic
platform) via the jar avenue; recorded as an open lead, not guessed.

**Open passes (Tekkit):** 1) IC2 values are extracted (`_harvest/ic2/`) — the remaining IC2 pass is
mapping them into roster rows; 2) Railcraft recipe values **done 2026-10-03** (`_harvest/railcraft-values.tsv`; 7.x registers crafters in code, not data JSONs, and the constants live in the API submodule, cloned at its exact pin); 3) RedPower-successor
names measured (above) — the block/ore registration list is next; 4) Forestry fluid constants
measured; process numbers next; 5) MFR fluids measured; the item/material registry pass is next;
6) the pack-manifest / Jaffa lead if the feature is picked.

## 7. The sweep status — every reference, and what remains (2026-10-03)

*"wait, did you go over all the mods? you might want to do that before rust invention"* — this
table is the answer, and **no Rust for the materials library is written until every row reads
complete or nothing**.

### A. The cloned trees

| mod | status | what remains |
|---|---|---|
| Create | PROCESS CHAINS DONE (`_harvest/create-processes.md`) | — (compat cross-refs noted; tech roster waits for the final pass) |
| TinkersConstruct | JSONs VERIFIED (`_harvest/stage-a-passes.md`): 469 files → 161 materials | — (real subset confirmed; fabricated refused; hepatizon/nicrosil candidates pending composition) |
| Create-Diesel-Generators | DONE (`_harvest/stage-a-passes.md`) | — (refining chain measured) |
| create-metalwork | DONE (`_harvest/stage-a-passes.md`) | — (casting/alloy recipes measured) |
| Rubberworks | DONE (`_harvest/stage-a-passes.md`) | — (sapping → rubber chain measured) |
| GregTech-Modern | CONDITIONS EXTRACTED (`_harvest/gt-conditions.tsv`: 143 temps + 45 hazards; `gt-no-composition.txt`: 134 ids) | — (roster condensation later) |
| create-new-age | DONE (scope = energy) | corium + radiation; nothing further |
| SteamPowered | DONE (tiny) | — |
| createsifter | DONE (`_harvest/stage-a-passes.md`) | — (sift outputs measured) |
| DenseOres | DONE (dense variants only) | — |
| optical | EMPTY CLONE | jar route if its content ever matters |
| TerraFirmaCraft | DONE (`_harvest/stage-a-passes.md`): bloomery/blast furnace/anvil/heating/welding/casting sampled; Grade enum found | — (tech roster waits for final pass) |
| BigGlobe | IDEA SOURCE | worldgen; no materials |
| lithostitched | IDEA SOURCE | worldgen; no materials |
| IC2 (jar mirror) | MAPPED (`_harvest/ic2/ic2-roster.md`) | — (kept/refused groups read; machines = process evidence) |
| IC2Classic | API ONLY | — |
| BuildCraft | DONE (fluids, gears, chipsets/facades read — no new earth materials) | — |
| Railcraft | VALUES MEASURED (`_harvest/railcraft-values.tsv`; API submodule cloned at its exact pin) | — |
| ComputerCraft | DONE (no new materials) | — |
| ProjectRed | DONE (registration measured `_harvest/stage-a-passes.md`) | — (no alloy furnace in PR; electrotine refused) |
| BluePower | DONE (`_harvest/stage-a-passes.md`) | — (alloy furnace ratios + flax/indigo read; real dopants [NS]) |
| EE 3 | DONE (magic deferred) | — |
| Forestry | DONE (`_harvest/stage-a-passes.md`): 28 wood species, still 10→3, fermenter/squeezer/fabricator read | — (bee species = content) |
| MFR | DONE (`_harvest/stage-a-passes.md`): rubber/plastic/ceramic/sugar-charcoal registry read | — (plastic feedstock [NS]; no licence — no derivation) |
| LogisticsPipes | DONE (no new materials) | — |
| ironchest | DONE (no new materials) | — |

### B. The material-plausible additions — cloned and swept (2026-10-03)

**Cloned 2026-10-03 (hashes in `_harvest/clone-hashes.txt`) — sweeps done 2026-10-03
(`_harvest/stage-b-sweeps.md`):** DynamicTrees `8bf66d83` — framework, no own substances;
DynamicTreesPlus `f408a8c7` — 8 species (mushrooms, cacti incl. saguaro); Ecologics `cfd79749` —
coconut/coir, prickly pear, walnut, azalea, seashell aggregate, ice/snow bricks; Immersive-Weathering
`1bcc03b6` — the **weathering system** (iron exposed→weathered→**rusted**, cracked/mossy/sandy/snowy
stone, charring, soils loam/silt/permafrost, fulgurite + vitrified sand, soot/mortar/tallow/steel
wool); SeedDelight `8b77f68d` — acorn/pine nut/sunflower/rosehip/cherry crops + products;
Create-Pantographs-and-Wires `3e18707b` — **coal coke, crushed coke, graphite**, copper wire, iron
shapes, oxidising masts; Tidal `ceec9a30` — **no materials** (water/tide mechanics only).

Still jar-route only: `water-your-crops` (CurseForge) — nothing material pending; the other
Modrinth entries (`refined-fluid-substitution`, `stockbridge`, `create-pressure-gauges`) are
described in the C accounting.

### C. The rest of the references list — **accounted 2026-10-03**

The full per-URL table over `docs/references.md` (93 URL lines + header = 94 lines, the user's
count) lives in `_harvest/stage-c-verdicts.md`. Tally: **28 swept** (every materials-plausible
reference — including the nine cloned today: refinedstorage2, CreateExtraGauges,
create-factory-controller, Threaded-Trains, more-vertical-additions, Ez-Stock-Ticker,
Mobile-Packages, Repackaged, Calculator) · **6 described** (functional only) · **1 not read by
design** (TFMG) · **2 idea sources** · **1 empty clone** (optical) · **6 jar-route, nothing
pending** · **1 unread page** (Ratatouille) · **48 nothing** = **93** ✔.

**The sweep reads complete**: A complete, B resolved, C accounted — residual items are named, not
omissions. Per C-129 the Rust wait ends here; per C-132 the one technology pass runs before the
Rust schema work starts.

## 8. The technology roster — final pass (2026-10-03, after the complete sweep)

Method per the C-130/C-132 rulings: **one pass** over every swept reference; a technology is a
**rule that transforms matter**; rows **dedupe to one per technology**, citing every swept mod that
declares it (the old mod *and* its Create-addon descendant — the double-cover rule); every row
carries its **real-world counterpart**. Measured inputs→outputs come from the `_harvest/` artifacts;
`[NS]` marks values that still want their own real-world source. §8.3 sketches the progression model
(the cellular automata) this roster feeds — design, falsifiable, to be built after the materials Rust.

### 8.1 The rule library (deduped, by family)

| family | technology (rule) | real-world counterpart | proposed by (deduped) | measured example |
|---|---|---|---|---|
| heat | furnace firing / smelting | kiln & furnace pyrotechnology | TFC `heating` (883; 930 °C sample) · Create `blasting` 27 + `smelting` 37 · Calculator ReinforcedFurnace · GT `electric_furnace`, `arc_furnace`, `steam_solid_boiler`… · RS2 smelting (quartz → silicon, 200 t) | crushed ore → ingot; quartz → silicon |
| heat | coking | coke oven (coal carbonisation → coke + tar/creosote) | Railcraft coke oven (coal → coke + **500 mb creosote**, 1800 t; block 4500) · PAW **coal coke** (burn 3200) + crushed coke | coal → coke + creosote |
| heat | bloomery | bloomery iron | TFC `bloomery` (charcoal ×2 + 100 mb cast iron → raw iron bloom) | |
| heat | blast conversion | blast furnace / converter (+ slag) | Railcraft (iron → **steel + slag**, 1280 t/unit) · IC2 (iron → refined iron + `itemSlag`) · TFC (`flux` → pig iron) | iron → steel/refined iron + slag |
| heat | steam generation | steam engineering (boiler, turbine) | SteamPowered · Railcraft SteamBoiler/SteamTurbine · MFR SteamBoiler/SteamTurbine · GT steam boilers | water + fuel → steam |
| comminution | crushing | jaw/gyratory crusher | Create `crushing` (211) · IC2 macerator · Railcraft rock crusher (100 t) · metalwork crushing · GT `macerator`/`steam_macerator` | ore → crushed raw; coal → coal |
| comminution | milling / quern | millstone, quern | Create `milling` (246) · TFC `quern` (89) | wheat → flour; stone → sand |
| sizing | sifting / screening | sieving & classification | createsifter (meshes; pebbles, crushed rock, crushed_raw_* set) | gravel → crushed ore + nuggets |
| sizing | washing / splashing | ore washing (water dressing) | Create `splashing` (52: crushed → **9 nuggets + clay**) · IC2 ore washer (**1000 mb water**/crushed ore) | crushed ore → nuggets + byproduct |
| separation | centrifugation | centrifugal separation | IC2 `thermal_centrifuge` · GT `centrifuge`, `thermal_centrifuge` | |
| separation | electrolysis | electrolysis | GT `electrolyzer` · IC2 electrolyzed water | |
| separation | extraction | extraction (solvent/mechanical) | IC2 extractor · Calculator Extraction/PrecisionChamber | stone/dirt → circuits (chamber set) |
| separation | distillation / refining | fractional distillation | BuildCraft refinery (oil → fractions + tar) · CDG distillation (crude → **50 diesel + 50 gasoline** heated; 75/75 superheated) · GT naphtha/cracked fractions · Forestry still (**10 mb biomass → 3 mb bio-ethanol**) | |
| separation | fermentation | fermentation (biomass, ethanol) | Forestry fermenter (sapling/cane/mushroom → biomass) · CDG bulk_fermenting (fermentable + bone meal → **400 mb ethanol**) · GT fermented biomass | |
| forming | casting | mould casting | TFC `casting` (111; 100 mb + ceramic mould → head) · metalwork (**9000 mb → ingot**; brass 9000+9000 → 36000) · CDG casting | molten metal → ingot/tool head |
| forming | pressing / rolling | press, rolling mill | Create `pressing` (43; sheets) · IC2 metal former rolling · Railcraft rolling machine (rails: iron 8 … tungstensteel 48) · metalwork compacting | ingot → sheet/rail |
| forming | wire drawing / extrusion | wire drawing, extrusion | PAW copper wire · CDG `wire_cutting` · GT `wiremill` · IC2 metal former extruding | metal → wire |
| forming | hammering | sheet-metal hammering | CDG `hammering` (brass/copper/gold/iron sheet) | |
| forging | anvil work | blacksmith forging (ordered steps) | TFC `anvil` (281; draw/hit **rules**) + `welding` (87) · MFR AutoAnvil | |
| joining | welding | forge welding | TFC `welding` (87) | two inputs → one part |
| textiles | sewing / weaving | sewing, loom | TFC `sewing` (25), `loom` (5) · BluePower flax → fibre chain · SeedDelight crops · Tinkers string | |
| assembly | mechanical assembly | assembly line, mechanisms | Create `mechanical_crafting`/`sequenced_assembly` · RS2 processors · Calculator assembly tiers · PAW sequenced cantilever | parts → mechanism |
| agriculture | planting / harvesting | agriculture (field, orchard) | MFR Planter/Harvester/FruitPicker/Fertilizer · Forestry arboriculture + 28 species · SeedDelight crops · Ecologics (coconut, prickly pear, walnut) · DynamicTrees growth · TFC farming | seed → crop |
| agriculture | husbandry | animal husbandry | MFR Rancher/Breeder/Vet/Slaughterhouse/Grinder/MeatPacker · Forestry apiculture · Ecologics fauna | |
| agriculture | composting / peat | composting, peatland | MFR Composter · Forestry bog earth/peat | |
| arboriculture | sapping / tapping | latex tapping | Rubberworks `sapping` (**40 mb resin**/species; 500 mb → 2 rubber) · IC2 treetap (sticky resin) | tree → resin/latex |
| chemistry | tanning / scraping | leather tanning | TFC `scraping` (7) + barrel · Create leather | hide → leather |
| chemistry | soaking / brining / dyeing | vat chemistry | TFC `barrel` (331) · BluePower indigo + flax · GT dye set · MFR ceramic dye | |
| weathering | oxidation / patina | corrosion science | Immersive-Weathering (iron **exposed → weathered → rusted**; waxing) · Create copper weathering (`deploying` 167) · PAW oxidising masts | Fe → rust; Cu → patina |
| weathering | frost / freeze–thaw | frost & freeze–thaw weathering | Immersive-Weathering (`frost`, frosty glass, icicle, thin ice; snowy stone variants) | |
| pyrolysis | charring | pyrolysis → charcoal | Immersive-Weathering charred wood · GT charcoal byproducts + wood tar/vinegar · MFR sugar charcoal | wood → char |
| masonry | stone dressing | masonry | TFC `chisel` (513) · Create `cutting`/stonecutting · TFC `collapse` (525) | stone → dressed stone |
| glass | glassworking | glassblowing / float glass | TFC `glassworking` (60; molten glass **2400** density) · Forestry fabricator (electron tubes in **liquid glass**) · BluePower sapphire glass · Create glass | silica → glass |
| ceramics | knapping / moulds | ceramic tooling | TFC `knapping` (101), mould casting · CDG mould cutting (bar/bowl/chain/line) | |
| woodworking | joinery / carpentry | carpentry, impregnation | Forestry carpenter (**50 t + 500 mb honey → scented paneling**; 250 mb seed oil → impregnated casing) · Create `cutting` (32) | |
| electronics | silicon refining | metallurgical-grade silicon | RS2 (**quartz → silicon**, 200 t) · BluePower **boule = 8 coal + 8 sand** (carbothermal) · PR `rough_silicon_wafer` | SiO₂ + C → Si |
| electronics | wafer doping | semiconductor fabrication | BluePower red/blue **doped wafers** (dopants refused/minecraft — real dopants **[NS]**) | |
| electronics | circuit making / rework | PCB assembly, rework & repair | Calculator circuits (board/dirty/damaged) + Reassembly/Restoration/Processing/Analysing chambers · RS2 processor tiers | dirty/damaged → working circuit |
| instrumentation | gauges & control | instrumentation, process control | Create factory gauges (bonus goal) · create-factory-controller · Create-Ez-Stock-Ticker · Repackaged · ExtraGauges · create-pressure-gauges (**pipe/pump pressure diagnostics**) | |
| storage/logistics | warehousing & handling | warehousing, materials handling | RS2 (disks/network) · ironchest · MFR ItemRouter/LiquidRouter/DeepStorageUnit · stockbridge · LogisticsPipes · ComputerCraft (programmable control) | |
| construction | brick/block making | brickmaking | Create/BluePower/TFC brick sets · Immersive-Weathering (ice/snow bricks, seashell aggregate) | clay → brick |

### 8.2 What the dedupe proves (old mod ↔ Create-addon descendant)

Double-cover is real and now visible per row: coking is both Railcraft's and PAW's; crushing spans
Create/IC2/Railcraft/metalwork/GT; casting spans TFC/metalwork/CDG; distillation spans BuildCraft/CDG/
Forestry/GT; sapping spans Rubberworks/IC2; oxidation spans Immersive-Weathering/Create/PAW. Each is
**one rule** with multiple citations — the roster must never grow a second row for a descendant.
The Create addons that propose no new technologies (gauges, controllers, tickers, trains, vertical
belts, network tiers) are cited only under instrumentation/storage rows.

### 8.3 The progression model — a cellular automata (design sketch, falsifiable)

Per the C-130 ruling, the progression is to be **a cellular automata over tools and machines** —
not a tier list, and not a mass of material rows. Design, to be built after the materials Rust:

- **Cells = working positions** on a 3-D lattice (the world), each holding a **state**: the material
  present, its form (ore → crushed → concentrated → metal → melt → part), and the machine at that
  position. Materials' state alphabet is the Rust materials table's vocabulary (the reason this
  design waits on it).
- **Rules = the §8.1 rows**: each technology is a local transition `(machine, inputs, condition) →
  outputs`, where the condition is declared, exact data — heat (the TFC 0–1600 °C ladder), time,
  pressure/water. No rule fires without its condition; refusal, never approximation (the standing
  rule).
- **Neighbourhood**: a cell's transition may require neighbours (fuel, water, airflow, another
  machine) — the CA is local by construction, which is exactly what makes belts/pipes *carriers*
  (H-24.2) rather than hidden teleports.
- **Progression = reachability**: the "tech tree" is the set of states reachable from the starting
  alphabet (stone, wood, fire, hand) in `n` CA steps; an era is a reachable set, not a label. A tool
  or machine is *unlocked* when a rule can first fire, which is what makes tools/machines the
  progression — exactly the user's words.
- **What must exist first**: the materials table (states), the exact-conditions vocabulary (
  temperature, time, mass — the GT temperatures/hazards and TFC heat ladder are its first declared
  data), and one rule runner. The roster above is the rule library's table of contents.

### 8.4 Measured vs [NS] — the pass's honesty line

- **Measured:** TFC 6,431 recipe files/19 dirs + heat ladder + alloy ranges + anvil/welding/casting/
  bloomery/blast furnace; Create 2,933/20 dirs + full ore chain; IC2 14 machine tables + washer water
  + slag; Railcraft coke/creosote/slag + steel + crusher + rolling; BuildCraft oil fractions + tar;
  CDG refining (plant oil, ethanol, biodiesel, diesel/gasoline, distillation splits); Forestry 28
  species + 10→3 still + fermenter/squeezer/carpenter/fabricator; MFR 43 machines + fluids; GT 143
  temperatures + 45 hazards + 654 material ids + machine sample; BluePower alloy furnace ratios +
  boule + doping; RS2 silicon/processors; Calculator circuits/coal process; weathering system;
  sapping; sifting; casting; all cited above.
- **[NS] (needs its own real-world source before a value hardens):** temperature unit of the GT
  builder (°C vs K); Forestry fermenter yields (config [D]); the TFC ore-grade values; steelmaking
  numbers beyond Railcraft's; MFR plastic feedstock; wafer dopants; Jaffa specifics (pack jars);
  TFMG (not read, by design); Tinkers hepatizon/nicrosil compositions; Ratatouille (unread page);
  `optical` (empty clone).
- **Two rulings stand:** no minecraft progression (roles from real properties, C-127) and
  no rotational power model (shafts are physical parts; **the CA carries machines, not energy
  networks** — boilers/turbines/water wheels are machines and captured sources, per the S-01
  best guess awaiting the user).
