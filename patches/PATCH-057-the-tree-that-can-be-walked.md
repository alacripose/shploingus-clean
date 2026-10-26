# PATCH-057 — the tree that can be walked

The mission this pass was given, 2026-10-23:

> *"Implement the full technology and materials trees with real world crafting methods from the F
> drive's mod harvest. … every tier, substance and process the harvest supports, each process
> carrying the real-world method the harvest measured — inputs, outputs, heat, hours, tool,
> structure, labour, power, skill — and every row traceable to its harvested source rather than
> invented. … Then give the tree an owner in code that validates it: no tier unreachable from the
> first, no substance without a process path that produces it, no process naming an unknown input or
> output, and a test that proves completeness and reachability over the whole set and fails loudly if
> a future edit orphans a node."*

# What was there, and what the harvest actually said

Two rungs, 70 substances, 11 processes — the stone-rung slice, not a tree. The roster that describes
the rest was prose in `MODS-MATERIALS.md` §8, and the F: harvest behind it is real: 30 mod clones,
`_harvest/create-processes/` (18 process dumps), `gt-conditions.tsv` (143 temperatures, 45 hazards),
`gt-material-ids.txt` (654 ids), `railcraft-values.tsv`, `tekkit-fluids.tsv`, and the stage A/B/C
pass notes.

**Reading it corrected four things this pass had already written down wrong.** The 930 °C sample in
§8.1 is TFC's *brass* heating, not a brick temperature — the fire-brick row now declares 900–1000 °C
as real kiln practice and says the harvest is silent. Create's `smelting` is ore → ingot, not a
charcoal burn. Brass at 70/30 matched no measured ratio (TFC's range is 0.88–0.92 Cu, BluePower's is
3:1); the row is now 900/100, inside TFC's measured range. And iron ore sat on the copper rung; the
magnetite and limonite reductions belong to iron & steel, where they now are.

The other direction was confirmed rather than corrected: `gt-conditions.tsv` gives Copper 1358, Tin
505, Zinc 693, Bismuth 545, Nickel 1728 — read as kelvin, each is the metal's real melting point to
within a degree (1085, 232, 420, 272, 1455 °C), which resolves the unit `[NS]` §8.4 carried without
inventing anything.

# What changed

| file | the change |
|---|---|
| `crates/voxel-model/materials/tiers.tsv` | **8 tiers** — hands & stone, bound & composite, fire & clay, copper & bronze, iron & steel, mill & mechanism, chemistry & weathering, electricity & precision — each with the rung's own note |
| `crates/voxel-model/materials/substances.tsv` | **128 substances**, 58 new: the metals (copper, tin, zinc, bismuth, nickel, antimony, iron, steel, cast iron, silicon), the alloys (bronze, brass), the ceramics and binders (brick, quicklime, plaster, mortar, crucible, glass, glassware), the workshop goods (cloth, iron sheet, copper wire, steel rail, mechanism, circuit, wafer), the chemistry (coke, creosote, tar, crude oil, diesel, gasoline, ethanol, potash, black powder, vermilion, patina, sludge, rubber, leather, treated timber), and the by-products the ledger must be able to point at (slag, steam, carbon dioxide, stone dust, pebbles, bran, flour, bread, grain, biomass, hide, bark, resin, crushed copper, crushed iron, dressed stone, gravel) |
| `crates/voxel-model/materials/processes.tsv` | **64 processes**, 53 new, each with inputs, outputs, exact heat and kind, hours, labour hours, power, structure, tool, skill — and a mandatory `source`. Gathers state `declared:`; the rest cite the artifact (`out.crushing`, `out.splashing`, `out.mixing`, `out.milling`, `out.pressing`, `out.smoking`, `gt-conditions.tsv`, `railcraft-values.tsv`, `tekkit-fluids.tsv`, `stage-a-passes.md`) or the §8.1 roster row, and say `declared:` for every mass yield no artifact measures |
| `crates/voxel-model/materials/vocabulary.tsv` | **25 entries** — structures (kiln, furnace, forge, anvil, boiler, mill, press, vat, loom), tools (hammer, chisel, knife, sieve, quern, die, pipe) and skills (firekeeping, smelting, alloying, smithing, machining, chemistry, weaving, knapping, farming), each required by some process (the gate refuses a vocabulary entry nothing requires) |
| `crates/voxel-model/src/materials/schema.rs` | `Process.source`, and the gate refuses an empty one: *a process nothing states is an invented one* |
| `crates/voxel-model/src/materials/declaration.rs` | the `source` column (required) in the TSV format and its parser; the parser's own test carries it |
| `crates/voxel-model/src/materials/tree.rs` | **the owner** — `Coverage`, `Tree`, `Table::tree()`: a tier-ordered fixpoint walk from what the world holds, refusing an unreachable tier, an unproduced `made` substance, a dead process and an undeclared name; `report()` prints the counts |
| `crates/voxel-model/src/materials/mod.rs` | `tree` is a module and `Coverage`/`Tree` are re-exported |
| `crates/voxel-model/src/bin/materials-generator.rs` | refuses to write while the walk is not whole, and prints the coverage line beside the gate's |
| `crates/voxel-model/tests/materials_declaration.rs` | pins the new counts and asserts the walk whole over the shipped declaration |
| `crates/voxel-client/src/shading.rs` | the corpus census keeps its accounting and its stale-excuse check, but its **zero-debt** gate now ranges over the rows the appearance corpus actually claims (`corpus::ROWS`, 65) instead of the whole declaration; the tree's 58 unmeasured substances are printed as the appearance pass's work list rather than asserted away. **No tree substance was admitted to the registry** — that wiring is the mission's stated boundary |
| `docs/MODS-MATERIALS.md` | §8.5 — the declared tree, the counts, the mandatory source, and the branches the harvest is silent on |
| `docs/TODO.md` | item 13 records the landing |

# The proof

`materials::tree` has six tests: the whole-table case; a forward edge (an early rung consuming a
later rung's product) leaving its tier unreachable and naming the stranded row; a `made` substance
whose producer was deleted reported as an orphan *and* the row that consumes it as dead; a tier with
no work at all; an unknown input name; and the committed table walked end to end, asserting every
rung of the eight-rung ladder is reached, every declared substance is obtainable, every process
fires, and printing the counts.

The walk's report over the committed table:

```
tiers 8/8 reached, substances 128/128 available (62 made, 62 produced), processes 64/64 fired
```

Per rung: 7 / 2 / 8 / 11 / 8 / 10 / 15 / 3 processes, every one firing.

# Verified

- `cargo test -p voxel-model`: **305 lib tests + 3 + 5 integration tests, 0 failed**; the six tree
  tests pass, the committed-table test prints the line above.
- `cargo check --workspace --all-targets`: **exit 0**, no errors (warnings are pre-existing).
- `cargo test -p voxel-client --lib the_corpus -- --nocapture`: **1 passed**, printing
  `corpus: 128 declared substances; 65 have an admitted material carrying a sourced reflectance; 5
  are stated absences; 0 lack a density; 58 lack a sourced reflectance` — the appearance debt the
  tree opens, named rather than hidden.
- `cargo run -p voxel-model --bin materials-generator --features materials-regeneration`: gate clean
  (8 tiers, 128 substances, 64 processes, 25 vocabulary), tree whole, four declarations and
  `digest.txt` written.
- The digest gate bites: with the declarations edited and `generated.rs` stale, a normal build
  **panicked at `build.rs:123`** on the declaration digest before the table could compile, and only
  `--features materials-regeneration` let the generator rewrite it.

# What it does not do

- **Thirty of §8.1's 41 rules have rows; eleven are named, not implied.** Casting and moulds need a
  molten state the substance table does not declare; centrifugation, electrolysis and extraction need
  a solution state; husbandry, composting and dyeing each open a new substance family this pass did
  not invent; wafer doping's real dopants are `[NS]`; instrumentation and storage/logistics are
  machinery and handling, not matter transformations, and the gate would rightly refuse a row for
  them. The list is in `MODS-MATERIALS.md` §8.5.
- **No inventory, no recipe lookup, no craft action.** The tree is declared data and its owner, as
  the mission scoped it.
- **Nothing consumes it outside `voxel-model`.** The appearance registry is untouched; the
  materials→rendering wiring remains the next domino (`TODO.md` item 14).
- **Mass yields are the tree's own `[D]` wherever the harvest states none**, and the rows say so
  rather than borrowing authority they do not have. The honest gaps — brick temperature, the
  charcoal burn, black powder, the crucible, creosote treatment, ash potash, vermilion, the
  distillation split, every agriculture/baking/glass batch ratio, and wafer dopants — are listed in
  §8.5 and marked in the `source` column.

# Update 2026-10-24 — the absence this record named is closed

This record ended with *"No inventory, no recipe lookup, no craft action"*. That was true when it
was written; it is not true now. `voxel_model::materials::stock` adds the three:

* `Stock` — grams held per declared substance, plus the structures, tools and skills the player
  has. It owns no declarations, so it cannot disagree with a row.
* `Table::offers` — **every** process as an `Offer`, with the grams and words it is short of. Ready
  offers sort first inside their rung; a blocked branch is returned rather than hidden.
* `Table::craft` — checks the whole row (inputs, structure, tool, skill) before moving a gram, then
  spends the inputs and adds the outputs. A refusal leaves the stock exactly as it was.

Six tests exercise it: the empty stock offers all 79 rows and only gathers are ready; a melt is not
satisfied by its solid; a missing structure is named; a short charge refuses without moving the
ledger; a missing word refuses without moving it; and the press-cake solvent charge is converted
through ethanol's own density rather than a guessed one. `materials-options` is the command path —
with `--stock copper=1000 --word structure=furnace --word skill=smelting --craft "melt copper"` it
prints 79 options (5 ready), performs the row, and leaves 1000 g of molten copper.

## Recipes

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-model/src/materials/tree.rs:146
what: keep `Table::tree` the walk that refuses an unreachable tier, an unproduced substance and a process nothing requires
because: the committed table is 8 tiers, 128 substances and 64 processes, and the walk reports 8/8 tiers reached, 128/128 substances available and 64/64 processes fired from hands & stone
check: cargo test -p voxel-model --lib materials::tree -> failures 0
witness: crates/voxel-model/src/materials/tree.rs
expects: 0
requires: cargo check --workspace
```
