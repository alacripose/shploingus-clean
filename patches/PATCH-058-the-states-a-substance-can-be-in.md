# PATCH-058 — the states a substance can be in

The mission this pass was given, 2026-10-23:

> *"Add those state dimensions, then the rows that need them … a molten or dissolved input must still
> be produced by something reachable, the coverage line must still print, and the reachability proof
> must fail loudly if a state makes a node unobtainable. … Do not invent a number to make a branch
> close, and do not force a rule into a row the harvest does not support."*

# Why a state axis, and not just more rows

Five of §8.1's eleven unrowed rules were unrowed for one reason: the substance table knew `Mass`,
`Volume`, `Count` and `Gas`, but a **melt** and a **solution** were not states a row could have.
Casting needs molten metal, glassblowing needs a gather, alloying is a mixing of melts, leaching
carries a solid in a liquor, and a centrifuge and a cell both work on that liquor. Adding the rows
without the state would have made `molten_copper` a `Volume` row that nothing could distinguish from
a bottle of copper-coloured water.

`State` is now a declared axis: **`Solid | Liquid | Molten | Solution | Gas`**. The unit and the
state must agree — a solid is never counted by volume, a liquid/melt/solution always is, and only a
gas uses the gas unit and the atmosphere sink. `Molten` and `Solution` are **derived** states and
their row must name the solid it comes from in a new `phase_of` column:

* **molten copper** is copper at 1085 °C (`phase_of: copper`);
* **pregnant liquor** is crushed ore's copper carried in water (`phase_of: crushed_copper`);
* **spent liquor** is the same liquor after the cell (`phase_of: crushed_copper`).

The gate refuses five things a state can get wrong: a state that disagrees with its unit; a derived
row with no `phase_of`; a `phase_of` that names itself, an undeclared substance, or a row that is not
a solid; a derived row held by the world (nothing mines a melt); and a **molten row that no process
melts** — the producing process must consume that solid, or (for an alloy) a molten constituent. That
last rule is the state's teeth: it is a declaration-time proof that every melt is actually melted.

# The rows the states unlocked

| rule | row(s) | the measured thing it rests on |
|---|---|---|
| forming/casting | `melt copper`, `melt tin`, `melt zinc`, `melt iron`, `melt glass`, `cast bronze`, `cast brass` | TFC casting 111 (100 mb + ceramic mould → head); GT's metal temperatures read as kelvin (Copper 1358 K = 1085 °C, Tin 505 = 232, Zinc 693 = 420); TFC glassworking 60 (molten glass 2400) |
| forming/casting (moulds) | vocabulary `mould`, skill `casting`; the cast rows require them | the mould is reusable equipment, so it is a **structure**, not a consumed row |
| alloying | `alloy bronze` and `alloy brass` now consume the melts and produce a melt | TFC's measured ranges (bronze 0.88–0.92 Cu + 0.08–0.12 Sn; brass 0.88–0.92 Cu + 0.08–0.12 Zn) |
| heat/blast conversion | `cast iron` now consumes `molten_iron` + coke | Railcraft blast furnace 1280 t, one slag; TFC cast iron |
| glassblowing | `blow glass` now consumes `molten_glass` | TFC glassworking 60 |
| sizing/washing + extraction | `leach copper ore` → pregnant liquor + residue | IC2 ore washer (1000 mb water per crushed ore); the carry is declared `[D]` |
| separation/centrifugation | `centrifuge the leach` → clear liquor + sludge | GT centrifuge, IC2 thermal_centrifuge; the fines are declared `[D]` |
| separation/electrolysis | `electrowin copper` → copper + spent liquor + sludge | GT electrolyzer; the cell's yield is declared `[D]` against real electrowinning |
| separation/extraction | `press seed oil` → oil + cake | Forestry squeezer (250 mb seed oil → impregnated casing); the press yield is declared `[D]` |

`impregnate timber` now takes both preservatives the harvest measures — creosote (the coke oven's
500 mb) and 20 g of pressed seed oil — instead of creosote alone.

# The counts and the proof

```
gate clean: 8 tiers, 141 substances, 75 processes, 29 vocabulary entries
tree whole: tiers 8/8 reached, substances 141/141 available (75 made, 75 produced), processes 75/75 fired
```

Per rung: 7 / 2 / 8 / 16 / 9 / 11 / 18 / 4. The thirteen new substances are seven melts, three
liquors, the leach residue, pressed seed oil and its cake; the eleven new processes are the five
melts, the two casts, the leach, the centrifuge, the cell and the press.

The walk has no exemption for a state. A new test, `a_molten_row_with_no_melt_is_unreachable`,
declares a melt that nothing produces and a cast that wants it, and asserts the report names both
(`molten_copper: declared `made`` and `cast the melt: dead`) rather than skipping the state. Three
gate tests pin the state rules: a melt with no `phase_of`, a `phase_of` that is not a solid, and a
solid counted by volume.

# What still has no row, and why

Of §8.1's 41 rules, **35** now have rows. The six remaining are named in `MODS-MATERIALS.md` §8.5:
**husbandry, composting and dyeing** (each opens a substance family — meat, compost, dyed cloth —
that this pass did not invent), **wafer doping** (the harvest's dopants are refused; the real ones
are `[NS]`), and **instrumentation** and **storage/logistics** (machines and handling, not matter
transformations; the gate balances grams and would rightly refuse a row for a gauge or a warehouse).
**Extraction** has its mechanical half only: no solvent row is declared, so the solvent half of that
rule is not a row.

# The census pin that moved, stated plainly

`crates/voxel-client/src/shading.rs` contains `the_corpus_names_what_each_unadmitted_substance_is_missing`,
which was PATCH-057's one change outside `voxel-model`. **What it now asserts:**

1. Every declared substance lands in exactly one column of the census — admitted, excused
   (`NO_REFLECTANCE`), missing a density, or missing a sourced reflectance — and the counts must add
   up (`accounted == declared.substances.len()`).
2. An admitted material must carry a sourced reflectance.
3. An excuse must have a reason, and an excuse may not outlive the admission it explained.
4. **New:** every row the appearance corpus claims (`voxel_shapers::corpus::ROWS`, 65 rows) must be
   admitted with a reflectance — `corpus_debt.is_empty()`.
5. **New:** no declared substance may lack a density (`missing_density.is_empty()`).

**It does weaken the old pin, and the weakening is real.** The old assertion was
`missing_density.is_empty() && missing_source.is_empty()` over the *whole declaration*: any new row
without an appearance failed the client's test. That was correct while the declaration was the
appearance corpus, and is wrong now that the declaration deliberately carries a technology tree with
71 substances the appearance campaign never measured. Keeping it would have forced one of two
dishonest moves — invent reflectance rows, or admit 76 unmeasured substances to the registry — both
of which the mission forbids. So the universal "no debt" is gone; what replaces it is the
corpus-scoped gate (65/65 claimed rows admitted, which is the appearance campaign's actual promise),
the density rule, the accounting rule, and the printed debt:

```
corpus: 141 declared substances; 65 have an admitted material carrying a sourced reflectance; 5 are
stated absences; 0 lack a density; 71 lack a sourced reflectance
```

**The cost, named:** a future substance that *should* be admitted and is not will be printed as debt
but will **not** fail this test. The tree's own proof still fails if it is unobtainable, but
appearance coverage is now tracked, not gated, for anything outside `corpus::ROWS`.

# Verified

- `cargo test -p voxel-model`: **309 lib + 3 + 5 integration, 0 failed**; the seven tree tests pass
  and print the coverage line above.
- `cargo run -p voxel-model --bin materials-generator --features materials-regeneration`: gate clean,
  tree whole, table and `digest.txt` rewritten.
- `cargo test -p voxel-client --lib the_corpus -- --nocapture`: passed, printing the debt line above.
- `cargo check --workspace --all-targets`: **exit 0**, no errors.
- `cargo clippy -p voxel-model --lib --tests`: **exit 0**.
- `cargo test -p voxel-client --lib`: **532 passed, 2 ignored, 0 failed**.

# What it does not do

- **No appearance wiring.** The 71 unmeasured substances are printed as debt, not admitted.

# Update 2026-10-24 — the states now hold at the lookup and the act

This record is dated: it ended by saying there was no inventory, no recipe lookup and no craft
action. That gap is closed in `crates/voxel-model/src/materials/stock.rs` (see PATCH-057's update),
and the state axis is enforced there, not only at declaration time:

* `Table::offers` resolves each input through its own declared row and unit. Holding solid `copper`
  or solid `tin` does not satisfy `molten_copper` or `molten_tin`, so `alloy bronze` stays blocked
  with `molten_copper` named in its shortfall. The command path prints both melts; the stock test
  asserts `molten_copper` and that the alloy is not ready, not `molten_tin` by name.
* `Table::craft` checks the same rows before spending anything, so a craft cannot consume the solid
  in place of the melt, and a refused craft leaves the stock untouched.
* `a_melt_is_not_a_stand_in_for_the_solid_it_came_from` proves the first by asserting
  `molten_copper` specifically; the short-charge test proves the second.

The state rows, the gate and the tree walk are unchanged. What the update adds is a caller-facing
path where the state distinctions actually decide an offer.

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-model/src/materials/stock.rs:181
what: offer every declared process against a stock, naming the grams and the structure, tool or skill it lacks
because: alloy bronze consumes molten_copper and molten_tin, and holding copper or tin does not satisfy either; the state axis has to decide the offer, not just the declaration
check: cargo test -p voxel-model --lib materials::stock -> failures 0
witness: crates/voxel-model/src/materials/stock.rs
expects: 0
requires: cargo check --workspace --all-targets
```
- **The solvent half of extraction is still unrowed**, because no solvent is declared; and the six
  rules above remain named absences rather than fabricated rows.

The state rows are now readable through the same stock/offer/craft surface as every other row:
their units convert through their own densities, and a melt that is not held is a named shortfall
rather than a silently missing option.
