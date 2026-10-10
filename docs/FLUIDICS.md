# FLUIDICS — the one flow model, and its lattice

Authority: the user's redirects of 2026-10-04 (`LEDGER.md` **C-152** — fluidics first, then the
atmosphere working and the world shapers ticking, materials later; **no Minecraft fluids**;
**water is not a mineable block**; **clouds emergent**; **rain as animated 256-level
subvoxels**), the fluid rulings of `HYPERGRILLING.md` rounds 22–24 (H-22.1 one fluid of
components, pressure-driven; H-23.1 species × phase × reservoir × components; H-23.2 fluids
are substance rows; H-24.1 `porosity`/`fluid_capacity` with the existing refusal; H-24.2 one
flow model with pipes as carriers; H-24.6 storage shape, behavior blocks for what exceeds
it, gas in scope), the F1 exact/approximate boundary (`DESIGN-SPEC.md` §2.1,
`voxel-model::conservation`), and the vertical-axis ruling (**z is up**, C-138). **C-153** adopts the PDF as the implemented
model — the six-neighbour **cellular automaton**, replacing the v1 column ("we're building a
cellular automata not slicing it up") — and records the **256-level fill** as the display
interface: `Cell::fill_level`, the value the appearance layer animates the subvoxels from.

The **state-machine mechanism** is ala-cities —
`references/Docs-A/ala-cities/src/matter/fluids.rs`,
`references/Docs-A/ala-cities/src/world/hydrology.rs`, and
`docs/design_reference/docs/superpowers/plans/2026-09-24-world-generator-core.md` Task 6.
The **flow physics** is the user's `docs/fluidics_pressure_and_water_ingress.pdf`, adopted
**over** the city builder (§7). Both are **reference, never authority** (C-48/C-49): the
mechanism is cited, the code is never copied, and every number this tree declares is this
tree's.

---

## 1. What exists today (measured)

| what | where | what it is | does it move? |
|---|---|---|---|
| water as voxels | `column.rs` (`fy <= sea`) | `MaterialId(1)`, terrain-derived | no |
| the water ledger | `water.rs` | three reservoirs (ocean / surface / groundwater), closure at one machine epsilon (C-94.8) | a budget over derived states, not a flow |
| standing water | `hydrology.rs` | priority-flood depth, **metres of depth** | no tick, deliberately |
| rain onset | `weather.rs` | *when* rain starts, from tick noise (C-94) | ticked, not yet deposited |
| the rows | `materials/substances.tsv` | `porosity` / `fluid_capacity` columns, empty; `FluidCapacityExceedsPorosity` refuses a capacity above porosity | nothing reads them yet |
| the vocabulary | `voxel-model::matter`, `world::change::FluidAddress` | `FluidSpecies` (8) × `Phase` → `ReservoirAccount`; a fluid addressed by name/species/phase/coord | addresses a change; no change flows |

| the fluid lattice | `voxel-shapers::fluidics` | six-neighbour cellular automaton: hydraulic head, Darcy flux, retention and `K(S)` with hysteresis, exact transfer plans, the 256 fill level | **yes** — one step is the ×4 rung's; `dt` is passed by the caller |
| the world's fluid state | `voxel-shapers::fluid_state` | ocean / lake / karst as a pure query (`FluidKind` × species × phase × `FillLevel`), the edit refusal, `fluid_at` / `fill_level_at` | a derived query, no tick of its own; rain deposits through the weather crossing in `water.rs` |

**Fluid movement now exists as a simulation, not yet as world state.** `fluidics.rs`'s
six-neighbour lattice moves water between cells (§3; C-153). Water in the *world* now carries a
fluid state too: `fluid_state.rs` answers ocean / lake / karst at a point as pure derived state
(C-154), the edit path refuses any edit that targets a fluid rather than deleting it, and rain
deposits through the weather family's declared crossing. Nothing stores the lattice (C-2): the
lattice is the moving model, and standing water is resolved from terrain and hydrology at the
address asked about. The client's `is_water` still asks the material record alone — lakes are
not swum in yet; the seam is named in `game.rs`.

## 2. What the reference declares (the mechanism this tree adapts)

* **The roster is closed**: `Water, Groundwater, Rain, Ice, Steam, Oil, Lava, Gas`, each
  with a valid phase and a `ReservoirAccount` (`WaterCycle, Oil, Lava, Gas`).
* **A cell is a record**: coord, species, volume `Rational`, mass `Rational`, pressure and
  temperature and flow potential as **fixed-point at scale 1000**, phase, dissolved
  components (salinity), carrier (`ContainerId` — the pipe seam), source/sink account.
* **The water cycle holds one cell per coord**, and a `transfer` moves a cell between
  **adjacent** coords when `FlowExchange` evidence authorises it. The evidence is
  canonically ordered (origin/neighbour by coordinate), carries a `signed_potential`, a
  `magnitude` and a **`tolerance`**, and a magnitude at or below tolerance authorises
  **no** flow (`NoCanonicalFlow`). The pressures are validated against the cells'
  own, so evidence cannot describe a move the cells disagree with.
* **The hydrology stage** computes precipitation → `infiltration + runoff ==
  precipitation`, `recharge` → `discharge + water_availability == recharge`, with
  `recharge ≤ infiltration` and `discharge ≤ recharge` checked **by refusal**; every
  sample carries process id, source ids and a digest chain. A chunk seam samples **the
  same global neighbour** from both sides.
* **The split is explicit**: mass and volume are exact `Rational`s; pressure, temperature
  and flow are fixed-point quantities that **declare scale and tolerance**. "No fluid
  state is inferred from a rendered material or a biome label."

## 3. The adopted model (v2 — the six-neighbour cellular automaton)

* **The lattice.** One voxel = one cell, on the world's own six-neighbour grid (`±x`, `±y`,
  `±z`, **z up**, C-138) — not a vertical column: the world is cubic chunks (H-24.8), and a
  model that cannot carry water sideways cannot carry an ocean. Each edge is the face between
  two adjacent cells, enumerated once from the lower coordinate, so a chunk seam is one edge
  and both sides sample the same global neighbour by construction.
* **The step is the PDF's recipe, in the PDF's order** (§7): boundary conditions →
  `Pc = retention_curve(S)` → `H = p/(ρg) + z` → `flux = −K_eff ∇H` → `S` by exact mass
  balance → recompute conductivity and capillary pressure. `dt` is passed by the caller: at
  the world clock's 20 Hz and the ×4 rung, one step is 1/5 s.
* **Cell.** A `Cell` carries the fluid (`Fluid`: address, density, viscosity, surface
  tension), the material (`PorousMaterial`: **permeability**, pore radius, contact cosine,
  retention, conductivity), exact volume and pore capacity, the signed gauge pressure, its
  wetting/drying branch, the wetting-front square `x_f` and its temperature. One fluid per
  cell: different fluids do not mix or displace (a named limit, not an accident).
* **Every physical property is sourced or refused.** `Permeability`, `Viscosity`,
  `SurfaceTension`, `PoreRadius`, `Temperature`, `ContactCosine` and every curve carry the
  source they came from; an empty source refuses by name. Porosity is not permeability — the
  flux needs a real `k`, and no material row has one yet, so real materials are refused until
  the rows land (test fixtures declare their own, as fixtures).
* **Exactness — and the arithmetic domain it forced.** Volume and mass are exact. The transfer
  plan is computed in `Rational`, **not** `Ratio`: `Ratio` is unsigned and *saturates*, and a
  scaling chain was measured silently collapsing to `1/1`, overfilling a cell (C-153.3).
  Pressure, head, flux, capillary pressure and the front are exact `Rational`s — suction is
  *negative*, so the unsigned `Ratio` cannot hold it; the reference's fixed-point-at-scale-1000
  is the declared scale at the **boundary** (`Cell::pressure_fixed`, which refuses a value the
  scale cannot represent rather than rounding).
* **The declared tolerance.** A head difference at or below **one millimetre** —
  `HEAD_TOLERANCE`, read from `voxel_model::conservation`'s `pressure and flow` family **at
  compile time** — authorises no flow. The reference mechanism (a magnitude at or below
  tolerance authorises no flow, at scale 1000) is cited; the number is this tree's.
* **The bounding, and why each face is scaled once.** Every transfer is computed from one
  snapshot; a giver's outgoing transfers are bounded by its own volume and a receiver's
  incoming by its free pore space, and each face is scaled by the **smaller** of those two
  factors, once. Scaling can neither create nor destroy volume; the result is a function of
  the input alone, not of insertion order. A cell that both gives and receives is bounded by
  the free space it has now — conservative, and declared as conservative.
* **The face conductance** between two cells is the series combination `2·K₁·K₂/(K₁+K₂)` —
  the PDF's resistance picture (a porous material as a hydraulic resistor). This is the
  model's declared discretisation, not a measured value; a zero conductivity on either side
  makes the face impermeable.
* **Retention, conductivity, hysteresis.** `Curve` is an exact fitted table — both ends
  declared (`S = 0` and `S = 1`), strictly ascending, interpolated exactly; a retention curve
  is nonnegative and reaches zero at saturation; a conductivity curve is a relative
  conductivity in `[0, 1]`. `Retention` carries **both** branches and a cell's branch follows
  the sign of its volume change, so the same saturation has two capillary pressures, as the
  PDF insists.
* **Boundary conditions** are imposed pressures (`Cell::impose_pressure`), deposits and
  drains; an imposed cell keeps its pressure while an interior cell's follows its own
  retention curve.
* **The 256-level fill** (the user's display directive). `Cell::fill_level` returns the
  **exact floor** of `saturation × 256` (`FILL_LEVELS`, read from `coord::DETAIL_SIDE`) —
  pure integer arithmetic, never rounded up, never stored as a grid (C-2). `FillLevel`
  refuses above 256 rather than clamping, and `surface_height_m` is exact. This is the value
  the appearance layer animates the stored 4×4×4 subvoxels from: rain, flowing water, molten
  metal (H-24.5's both bases animate).
* **The rung.** `fluidics::is_due_at(tick)` is `Stage::Hydrology`'s own declared cadence
  (×4, H-8.3; C-88.4), read from the stage, never restated.
* **Refusals, never clamps.** Over-capacity construction or deposit refuses by name
  (H-24.1 applied to state, not only to a row); a cell whose material has no permeability
  does not conduct; a curve that does not declare both ends refuses; a settle that hits its
  declared cap refuses rather than stopping silently.

## 4. The acceptance, and the PDF's own numbers

`crates/voxel-shapers/src/fluidics.rs` — **22 tests** (measured), the load-bearing ones:

* **the PDF's four worked examples, to the digit**: one metre of water = **9 810 Pa**; a 10 µm
  wetting pore = **14 400 Pa = 160/109 m** of head (between the PDF's 1.46 and its rounded
  1.47); the slab estimate = **1e-10 m³/s = 0.36 mL/hour**; Lucas–Washburn after one second =
  **9/25 000 m²**, i.e. a front between 18 and 19 mm — with **no square root taken anywhere**;
* **gravity drains from the top down, exactly**: a two-cell lattice's upper quarter reaches the
  bottom and the books close exactly (the settle cap is 8 moving passes per cell, declared);
* **the flow is the Darcy amount**: one metre of head over 1/5 s moves exactly `K·1·1/5` m³,
  and reversing the gradient reverses the flow;
* **the tolerance**: exactly one millimetre of head moves nothing;
* **suction pulls water in**: a dry cell's retention pressure is −14 700 Pa, and an overlying
  wet cell drains *into* it against gravity's direction;
* **hysteresis**: one saturation, two pressures, one per branch;
* **determinism**: the same lattice stepped in reverse insertion order is the same world;
* **a chunk seam is one edge**, and the flow crosses it;
* **scaling never creates volume**: a cell giving to two neighbours at once gives at most what
  it holds, and the total is conserved;
* **the 256 fill level** is the world's own scale, floored (a third of a cell = level 85,
  height 85/256 m) and refused above 256;
* a property with no source refuses by name; a curve must declare both ends and rise; a pressure
  the declared scale cannot represent refuses; the rung equals `Stage::Hydrology`'s.

## 5. What this does not claim

No mixture and no displacement: one fluid per cell. No gas yet (the same model is the next
landing), no dissolved components, no phase change, no thermal coupling — `μ(T)` and `γ(T)` are
`[NS]` and are not invented. No pipes as non-adjacent carriers (H-24.2: they arrive as their own
conductor law), no cracks as their own hydraulic domain, no surface spreading, no sorption.
Permeability is a **declared fixture** until the material rows carry a sourced `k`; the
retention/conductivity curves are declared fixtures likewise. Pressure and flow are carried
exact; the reference's fixed-point scale is the boundary. No rain deposit, no water-in-the-world
state, no integration with `hydrology.rs`'s depth or the water ledger. The face conductance is a
discretisation, declared as one. Every one of those is either named in §6 or refused by the
model rather than approximated.

## 6. Owed, named, in the order the user set

1. **The lattice — landed (C-153); its conductors are not.** Six-neighbour flow with the
   ingress reference's physics (§7) is in (§3). Pipes as carriers of the same model (H-24.2)
   and cracks as their own hydraulic domain are still owed, and the material rows still need a
   **sourced `k`** (porosity is not one).
2. **Water in the world — landed (C-154).** The ocean, lakes and karst answer as derived fluid
   state (`fluid_state.rs`); the edit path refuses to break or replace a fluid, on both the
   material half (`earthworks::hand_change`) and the derived half (`Pool::edit` refuses a lake
   that has no material voxel to break); rain deposits through the weather family's declared
   crossing (`WeatherAtmosphereClimate => Crossing(1)`, kg/m²/yr — one millimetre of annual
   rain). The client's `is_water` still asks the material record alone (lakes are not swum in
   yet — the seam is named in `game.rs`), and real motion remains the lattice's (item 1).
3. **The atmosphere stage.** Saturation from the weather/climate state, **emergent**
   cloud, rain drawn as animated 256-level subvoxels — `Cell::fill_level` is now the
   interface (§3); the animation itself is not started. The cloud reference is
   `docs/clouds_and_atmospheric_formation.pdf` (the mechanism is recorded in `LEDGER.md`
   C-152.4): saturation and RH as thermodynamic equilibrium, dry/moist lapse rates
   (~9.8 / ~4–7 °C/km), the lifting condensation level as cloud base, stability selecting
   cloud shape, CCN activation, and the cloud-water balance
   `formation + growth + transport − evaporation − precipitation − mixing losses`.
4. **The world shapers ticking.** The deferred tick carried to the shapers and the API
   (C-92.0 item 3, C-117's measurement) without the naive move's reshuffle.
5. **Behavior blocks.** A fluid whose behaviour exceeds the storage shape declares a
   behavior block (H-24.6); gas is in scope.

---

## 7. The ingress reference (the physics adopted over the city builder)

The user's `docs/fluidics_pressure_and_water_ingress.pdf` (added 2026-10-04) is the flow
physics' reference. The city builder remains the record for the roster, the cell record and
the tolerance mechanism; **pressure, ingress and unsaturated flow come from here**. The
mechanism, recorded so it does not live only in a PDF:

* **One driving potential.** Hydraulic head `H = p/(ρg) + z` — pressure head plus elevation
  head. Flow responds to the **gradient of the potential**, never to one pressure number.
* **Hydrostatic pressure.** `p_g = ρgh`: one metre of water ≈ 9.81 kPa, ten metres ≈ 98 kPa.
* **Saturated flow.** Darcy: `q = −(k/μ)(∇p − ρg)`, `K = kρg/μ`, i.e. `q = −K∇H`; the slab
  estimate `Q = (kA/μ)(Δp/L)` is the first-order engineering model. Permeability `k` (m²) is a
  property of the pore geometry, and **porosity alone does not tell how fast water moves** —
  two materials can share void volume and differ by orders of magnitude in `k`. (That is why
  v1's pair rule has no conductance yet, and why `porosity` is not a stand-in for `k`.)
* **Ingress routes.** Capillary imbibition, pressure-driven permeation, gravity-driven
  infiltration, surface spreading, sorption/molecular uptake, vapour transport, and
  cracks/joints as dominating defects — "treat these as separate hydraulic domains rather
  than averaging them blindly into one bulk property".
* **Capillary pressure.** Young–Laplace `Pc = p_air − p_water = 2γcosθ/r`; entry pressure for
  non-wetting liquids; capillary rise `h_cap = 2γcosθ/(ρgr)` — a **smaller pore pulls
  harder**. A 10 µm wetting pore ≈ **14.4 kPa ≈ 1.47 m of head**; Lucas–Washburn
  `L² = γrcosθ·t/(2μ)` gives ≈ 19 mm in one second in an ideal 10 µm pore.
* **Unsaturated materials are a different regime.** Saturation `S`, water pressure `p_w`, a
  retention relation `S(Pc)`, and an effective conductivity `K(S)` that can fall by orders of
  magnitude as the material dries (van Genuchten retention with Mualem conductivity); the
  sorptivity front `I(t) = S√t + A₀`; a Richards-type balance
  `storage change = divergence of [K(S) × hydraulic-head gradient]`; and **hysteresis** — the
  wetting path is not the drying path.
* **The simulation recipe.** State: `p_w, z, S, k, K(S), Pc(S), r_eff, θ, x_f, T`. Step:
  boundary conditions → `Pc` from `S` (or `r_eff`) → `H = p/(ρg) + z` → `flux = −K∇H` → update
  `S` by mass balance → recompute accessibility/conductivity/capillary pressure → repeat.
  Calibration order: **geometry → density/viscosity → permeability → retention → wetting →
  defects**.
* **References** carried for the sourcing discipline: Washburn 1921 (capillary flow),
  Richards 1931 (unsaturated continuum), Carman 1937 (granular beds), van Genuchten 1980
  (retention/conductivity), Hall 1980/1987 (brick and concrete sorptivity), Lukyanov et al.
  2020 (fibrous media), Thybring et al. 2022 (water in wood).

**What this changes for the lattice** (§6 item 1): the potential is `H`, not elevation alone;
`porosity` stays a storage property and the flux needs a **permeability**, which is `[NS]`
until sourced — the lattice inherits the same refusal discipline the material rows do.
Capillary entry decides *whether* a dry cell admits water at all (the water-ingress half);
`K(S)` and retention make drying and wetting different curves; cracks become their own
hydraulic domain when they land.
