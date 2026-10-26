# PATCH-060 — the solvent the harvest names

The mission this pass was given, 2026-10-23:

> *"Declare the solvent family and the extraction processes that need it, from the F: harvest as all
> the existing rows are … The solvent has to be produced by something already reachable and has to
> close the mass balance without inventing a yield; if the harvest supports a specific solvent, use it
> and say which artifact, and if it supports none, name that absence instead of picking a plausible
> one. Recovered solvent belongs on the output side if the harvest's method recovers it."*

# Which solvent, and on what evidence

**Ethanol**, and no new solvent row. The harvest names it as a substance twice —
`_harvest/gt-material-ids.txt` line 192 lists `ethanol` among GregTech's material ids, and
`_harvest/gt-conditions.tsv` line 132 carries its hazard line — and measures the process that makes
it: Forestry's still, **10 mb biomass → 3 mb bio-ethanol**, which the tree already rows as
`ferment biomass`. Ethanol is therefore reachable at rung 6 before any extraction runs, and it is
already a `Volume`/`Liquid` row whose own note says *"fuel and solvent: a use"*. The solvent family
was declared before this pass; the pass is the process that uses it.

**What the harvest does not support is the extraction itself.** No artifact measures a solvent
extraction: IC2's extractor is a heated press of rubber out of sticky resin (no liquid solvent, and
the tree rows that as `curdle rubber`), and Calculator's "Extraction" chamber is an item machine
(stone/dirt → circuits). So the row's numbers are this tree's own and its `source` says so in the
same line.

**Hexane is the honest absence.** It is the industrial oilseed solvent and it appears nowhere in
`_harvest/`; producing it would need a crude-oil fraction the measured distillation split
(`_harvest/tekkit-fluids.tsv`: diesel / gasoline / tar) does not contain. No hexane row exists, and
no row pretends one does.

# The row

```
extract the press cake   chemistry & weathering
  in : press_cake 700 g; ethanol 2000 mL (1580 g)          = 2280 g
  out: seed_oil 50 mL (46 g); spent_meal 661.9 g; ethanol 1990 mL (1572.1 g) = 2280 g
  78 °C material · 8 h · 1 labour-hour · structure vat · skill chemistry
```

The arithmetic, to the gram: the press leaves **50 g of oil in 700 g of cake** (7.1 %, the residual an
expeller really leaves), the wash recovers **92 %** of it — 46 g, the rest staying in the meal — and
the solvent is boiled back off at ethanol's own 78 °C boiling point, losing **10 mL (7.9 g)** to the
meal. The meal is 650 g of solids + 4 g of residual oil + 7.9 g of ethanol = **661.9 g**. The row
does not invent a closing figure: every mass on the output side has a name, and the two efficiencies
that are this tree's choice (92 % of the oil, 99.5 % of the solvent) are marked `[D]` in a `source`
that states the harvest measures neither.

One new substance: **`spent_meal`** — the defatted meal, 0.5 g/cm³ bulk, `no_consumer: animal feed: a
use`. `press_cake` lost its old excuse (*"animal feed: a use, once husbandry is a row"*), because it
now has a real consumer and an excuse may not outlive the admission it explained. The recovered
ethanol is the same row it borrowed, on the output side — a recycle, not a new substance.

# The audit the mission asked for

**Yes — five rows cited a measured ratio and did not reproduce it.** Each cited artifact states a
*count* or a *mod fluid volume*, not a mass ratio, and in each case the row settled its masses by
balance or by real practice without saying which. They now say:

| row | what the harvest states | what the row does | now marked |
|---|---|---|---|
| `coke coal` | coke oven: 1800 ticks, **500 mb creosote** per coal | 850 g coke + 140 mL (150 g) creosote | 500 mb is 535 g, which would leave 465 g of coke; the 85/15 split is `[D]` against real coking yields |
| `blast steel` | 1 iron ingot → 1 steel ingot, **1 slag**, 1280 ticks | 1000 g iron + 250 g coke → 900 steel + 300 slag | ingot-for-ingot and a slag count fix no masses; the three masses are `[D]` |
| `roll a rail` | rolling machine: **6 ingots → 16 rails** | 1000 g steel → 950 g rail | counts of toy items; the 950 g rail and its 5 % scale are `[D]` |
| `ferment biomass` | still: **10 mb → 3 mb** bio-ethanol | 1000 g biomass → 300 g ethanol | a volume ratio; the 300 g per kilogram and the sludge are `[D]` |
| `curdle rubber` | Rubberworks: **500 mb resin → 2 rubber** | 1000 g resin → 800 g rubber + 200 g sludge | a fluid-to-item pair; the 80/20 split is `[D]` |

No row claims a *mass* the harvest measured when the harvest measured a count, and §8.5 now states
that rule where it lists the measured examples. Every other row's numbers are either derived from
chemistry (the stoichiometric smelts, the melt densities, the calcination) or already marked `[D]`.

# The counts and the proof

```
gate clean: 8 tiers, 146 substances, 79 processes, 32 vocabulary entries
tree whole: tiers 8/8 reached, substances 146/146 available (80 made, 80 produced), processes 79/79 fired
```

Per rung: 7 / 2 / 8 / 16 / 9 / 13 / **20** / 4. The only pin moved is
`the_shipped_declaration_gates_clean` (145 → 146 substances, 78 → 79 processes, coverage with them),
with the tranche recorded in its comment. The gate, the walk, the digest gate, the four refusal kinds
and the state proofs are untouched.

# The public way in (the second question)

A crate outside `voxel-model` reads the tree through **`voxel_model::materials::generated::table()`**,
which returns `voxel_model::materials::schema::Table` — always compiled, not feature-gated — and then
calls `Table::tree()` (→ `materials::tree::Tree`, `coverage()`, `report()`, `defects()`),
`Table::gate()`, `Table::substance()`, `Table::grams_total()` or `Table::open_reasons()`.
`materials/mod.rs` re-exports only `tree::{Coverage, Tree}`, so `Table` itself must be named by its
`schema::` path or inferred. At the time of this pass there was no inventory, lookup or craft API —
the data and its owner were the whole surface.

# Update 2026-10-24 — the public way in now includes the act

The paragraph above describes this record's own pass and is kept as written. The absent surface it
names now exists in `materials::stock`: `Stock` holds grams and capability words, `Table::offers`
returns all 79 processes with their shortfalls, and `Table::craft` spends a row atomically. The
solvent row is not special-cased: `extract the press cake` converts its 2 000 mL ethanol charge
through ethanol's declared 0.79 g/mL, so the offer asks for 1 580 g of the same ethanol row the
still produces, and the craft returns the recovered 1 990 mL **recorded as 1 572.1 g of ethanol in
the stock** (the stock holds grams; 1 990 mL × 0.79 g/mL) rather than a fresh substance.
`materials-options` is the command path.

The act is at `crates/voxel-model/src/materials/stock.rs:235`; the check is `cargo test -p voxel-model --lib materials::stock`; the witness is `crates/voxel-model/src/materials/stock.rs`.

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-model/src/materials/stock.rs:235
what: perform one process atomically against a stock, converting every declared unit through its own density and refusing without moving the ledger
because: extract the press cake charges 2000 mL ethanol (1580 g) and recovers 1990 mL, so the solvent row is exact only if the craft uses the row's own unit and density and no invented mass
check: cargo test -p voxel-model --lib materials::stock -> failures 0
witness: crates/voxel-model/src/materials/stock.rs
expects: 0
requires: cargo check --workspace --all-targets
```

# Verified

- `cargo run -p voxel-model --bin materials-generator --features materials-regeneration` — gate clean,
  tree whole, table and `digest.txt` rewritten.
- `cargo test -p voxel-model` — **309 lib + 3 + 5 integration, 0 failed**.
- `cargo check -p voxel-model --all-targets` — exit 0.
- `cargo test -p voxel-client --lib the_corpus -- --nocapture` — passed, printing
  `corpus: 146 declared substances; 65 have an admitted material carrying a sourced reflectance; 5
  are stated absences; 0 lack a density; 76 lack a sourced reflectance`. `spent_meal` carries a real
  density and joins the printed debt rather than claiming a reflectance it does not have.

# What it does not do

- **No distillation row for the solvent strip.** The recovery happens in the same vessel at 78 °C and
  is part of this row; a separate still would be a second row for one operation.
- **No hexane, no supercritical CO₂, no solvent other than the one the harvest names.**
- **`spent_meal` is not wired to husbandry.** It is a stated feed, like `meat` is a stated food;
  wiring it would mean inventing the beast's ration split.
