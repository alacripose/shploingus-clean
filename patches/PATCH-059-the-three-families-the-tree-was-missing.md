# PATCH-059 — the three families the tree was missing

The mission this pass was given, 2026-10-23:

> *"Declare the families each needs and the processes that produce them, from the F: harvest
> artifacts exactly as the existing rows are — measured heat, hours, labour, power, structure, tool,
> skill, and a `source` that cites the artifact or says `declared:`/`[NS]` where it is silent. If one
> of these three turns out not to be a matter transformation the gate can balance, say so and leave
> it named in §8.5 rather than bending the row to fit; a correct absence beats an invented process."*

# Why these three, and not the other three

PATCH-058 left six of §8.1's 41 rules unrowed. Three of them — husbandry, composting, dyeing — were
not unrowed for want of a mechanism: the harvest measures the machines that run them, and the real
processes are as old as agriculture. They were unrowed because each one **opens a substance family**,
and a family is rows. This pass declares the rows.

The other three stay absent, and for the same reasons as before: **wafer doping** (the harvest's
dopants are refused, the real ones are `[NS]`, so the row would invent its own chemistry),
**instrumentation** and **storage/logistics** (gauges, routers and warehouses are machines and
handling, not matter transformations; the gate balances grams, and a row for a gauge would be a lie
about what it does).

# The rows

Four new substances, three new processes, three new vocabulary entries.

| row | tier | in → out | the measured thing it rests on |
|---|---|---|---|
| `raise livestock` | mill & mechanism | forage 2000 + water 1000 → meat 700 + hide 250 + manure 1500 + CO₂ 550 | §8.1 agriculture/husbandry (MFR Rancher / Breeder / Vet / Slaughterhouse / Grinder / MeatPacker, 43 machines; Ecologics fauna). The 2000 g meat fluid in `_harvest/stage-a-passes.md` is MFR's **block fluid**, not tissue — the `meat` row says so and declares 1.05 g/cm³. Feed ratio and breath are `declared: [D]` |
| `compost biomass` | mill & mechanism | biomass 1000 + manure 1000 + water 500 → compost 1900 + CO₂ 600 | §8.1 agriculture/composting (MFR Composter; Forestry bog earth/peat). The heap's self-heating to 55–65 °C is `[V]`; the 1900 g yield, the 600 g vent and the 30-day cycle are `declared: [D]` |
| `dye cloth` | chemistry & weathering | cloth 1000 + vermilion 20 + ash 50 → dyed_cloth 1070 | §8.1 chemistry/soaking-brining-dyeing (TFC barrel 331; GT dye set; MFR ceramic dye 16, measured in `_harvest/stage-a-passes.md`); `_harvest/stage-a-passes.md` also measures BluePower's indigo from a real plant. The 7 % bath add-on is `declared: [D]` |

New substances: `meat` (muscle tissue, 1.05 g/cm³, `no_consumer: food`), `manure` (the byre's dung,
0.9 g/cm³ bulk), `compost` (thermophilic, 0.6 g/cm³ bulk), `dyed_cloth` (0.32 g/cm³,
`no_consumer: a textile`). New vocabulary: structure `byre`, structure `compost heap`, skill
`husbandry`. `vat` and `farming` already existed and are now required by a second process each.

# The decision inside the numbers: the two rows that vent

A compost heap that returns its whole input is not a compost heap. The first draft of
`compost biomass` closed 2500 g of biomass, manure and water to 2500 g of compost, and that is a
claim physics rejects: composting burns dry matter to CO₂ and water vapour (30–50 % of dry mass).
The same is true of the beast: an animal that respires nothing is not an animal. Both rows now route
the loss through `carbon_dioxide`, the row that exists for exactly this and already holds
`ferment biomass`'s gas — 600 g for the heap, 550 g for the breath — and both state that the split is
`declared:`, because the harvest measures neither.

`plant grain` gained the other end of the heap: it now consumes 500 g of compost and 1900 g of
carbon dioxide to make 3500 g of grain (a crop's mass is mostly air and water). That is what gives
`compost` its consumer, and it makes the loop real — the heap's compost feeds the crop, the crop's
forage and the beast's dung feed the heap.

# The counts and the proof

```
gate clean: 8 tiers, 145 substances, 78 processes, 32 vocabulary entries
tree whole: tiers 8/8 reached, substances 145/145 available (79 made, 79 produced), processes 78/78 fired
```

Per rung: 7 / 2 / 8 / 16 / 9 / **13** / **19** / 4 (the two agricultural rows join mill & mechanism,
the dye vat joins chemistry & weathering). The walk, the four refusal kinds and the digest gate are
unchanged and strict: no state, family or tier has an exemption, and `the_committed_table_is_exactly_the_gated_declaration`
still fails if `generated.rs` and the rows disagree.

**No pin was loosened.** `the_shipped_declaration_gates_clean` moved 141 → 145 substances, 75 → 78
processes and 29 → 32 vocabulary entries, and its coverage assertion moved with them; the comment
above the counts records the tranche, as it does for every earlier one. Every assertion of wholeness,
consumer-honesty and balance is exactly as it was.

# What still has no row

Of §8.1's 41 rules, **38** have rows. Three remain, named in `MODS-MATERIALS.md` §8.5: wafer doping,
instrumentation, storage/logistics. **Extraction** has its mechanical half (pressed seed oil) but not
its solvent half, because no **solvent** row is declared — and the gate would refuse a solvent process
whose extractant appears on neither side of the balance.

# How a reader tells a harvested number from a declared one

- `source` starting with `harvest:` names the artifact (`_harvest/gt-conditions.tsv`,
  `_harvest/railcraft-values.tsv`, `_harvest/create-processes/out.crushing`,
  `_harvest/stage-a-passes.md`) or the §8.1 roster row the technology came from.
- `source` starting with `declared:` states the real-world fact and says the harvest is silent. The
  `note` then marks the individual number `[D]` (this tree's declared value) or `[V]` (verified
  real-world fact) — `[NS]` still marks a value that wants its own source.
- `provenance` repeats it at column level: `V` for a harvested/verified row, `D` for a declared one.

For this pass: the machines and the direction are harvested, the masses are declared, and each row
says which is which in the same line.

# Verified

- `cargo run -p voxel-model --bin materials-generator --features materials-regeneration` — gate clean,
  tree whole, table and `digest.txt` rewritten.
- `cargo test -p voxel-model` — **309 lib + 3 + 5 integration, 0 failed**.
- `cargo check -p voxel-model --all-targets` — exit 0 (two pre-existing unused-import warnings in
  unrelated crates' modules).
- `cargo test -p voxel-client --lib the_corpus -- --nocapture` — passed, printing
  `corpus: 145 declared substances; 65 have an admitted material carrying a sourced reflectance; 5
  are stated absences; 0 lack a density; 75 lack a sourced reflectance`. The four new rows raised the
  printed debt from 71 to 75 and broke nothing: they carry real densities, and the corpus-scoped gate
  (65/65 claimed rows) is untouched.

# What it does not do

- **No inventory, no recipe lookup, no craft action, at the time of this pass.** Husbandry is a
  row, not a mob. (The first half is closed on 2026-10-24; see the update below.)
- **No appearance wiring.** `meat`, `manure`, `compost` and `dyed_cloth` are printed as appearance
  debt like the other unmeasured rows; none claims a reflectance it does not have.
- **No solvent extraction.** The extractant row does not exist, so the rule stays in §8.5.

# Update 2026-10-24 — the rows are now options, and the first bullet is dated

This record's first bullet — *"No inventory, no recipe lookup, no craft action"* — was true when it
was written. `voxel_model::materials::stock` closes it: `Stock` holds grams and capability words,
`Table::offers` returns every process with its shortfall, and `Table::craft` performs a row
atomically. `raise livestock`, `compost biomass` and `dye cloth` are listed by `offers` like every
other row, so the three families this record added are reachable by a stock rather than only by the
gate. Only `raise livestock` was crafted end to end in that pass; `compost biomass` and `dye cloth`
were listed, not exercised.

**Husbandry is still a row, not a mob.** A craft yields meat, hide and manure; it does not create an
animal, and this update does not claim one. That remains PATCH-043's subject, and it is still
blocked on the external signer.

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-model/src/materials/stock.rs:181
what: offer the husbandry, composting and dyeing rows to a stock like every other process, naming the grams and words each is short of
because: the three families this record added had no caller-facing way to be chosen; the walk proved they were reachable, but the game could not ask what to do with a byre, forage and water
check: cargo test -p voxel-model --lib materials::stock -> failures 0
witness: crates/voxel-model/src/materials/stock.rs
expects: 0
requires: cargo check --workspace --all-targets
```
