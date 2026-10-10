# MISSING SYSTEMS — the shaper systems the audit found absent, inert, or uncalled

> **Status: research leads, not authority.** This file exists because `LEDGER.md`
> **C-174** (2026-10-09) measured that several of the declared shapers do not
> reach a voxel — either because their law is unwired, their dial sits past the
> model's own reach, or nothing calls them. It answers one question per system:
> **what primary source supplies the law, and where in this tree does it wire in?**
> It extends nothing. Where a decision is the user's, it says so and stops.
>
> The rule this file obeys is `docs/SHAPERS.md` §2: *no shaper is an unexplained
> pile of noise.* A system named here is a paper and an equation, never a vibe.

## 0. What "missing" means, measured

From C-174 and the call-site audit (`grep -rn "erode_patch\|erosion_droplets" crates/`,
`cargo run -p voxel-shapers --example cave-census`):

| Symptom | Measured | Where |
|---|---|---|
| **Erosion is absent from the terrain** | `erosion_droplets_per_column: 0`; `column::erode_patch` is called only by its own test and a timing example; `terrain::elevation_m` has **no erosion term** | `spec.rs`, `column.rs`, `terrain.rs` |
| **Caves are wired but nearly inert** | four of five rock classes' dissolution ceilings sit below the shipped `cave_threshold` 0.62 (0.319 / 0.347 / 0.376 / 0.622); a **global** water table makes open caves 0.9 % of vadoze rock | `caves.rs`, `cave-census` |
| **`delta`, `transition`, `process` have no caller** | the hits for the last two were `voxel-model::world::transition` and `std::process` | grep |
| **The scheduler has no production consumer** | `voxel-work::tick::Scheduler` and `voxel-shapers::process::ProcessScheduler` both exist and both plan by cadence; nothing drives them from the running world | `voxel-work/src/tick.rs`, `process.rs` |
| **Mountains almost never fire** | the mountain term is gated on `Convergent` boundaries, measured **5 of 5,168** samples | `relief-profile` |

## 1. The cadence tick — the runtime that is supposed to drive all of this

This is the "missing component" in the strict sense: a component that schedules
by cadence and has no consumer.

| Piece | Source | What is owed |
|---|---|---|
| The deferred tick | `LEDGER.md` **H-7.2 / H-8.3**; `voxel-client/src/clock.rs` (base 20 Hz, `MAX_CATCH_UP`) | the clock exists and is correct; what is absent is a **driver** that takes `Clock::advance`'s whole ticks and asks the due stages for their units |
| The stage scheduler | `voxel-work/src/tick.rs` (`Scheduler::plan`, `UnitOfWork`, `HALO_VOXELS`) | **landed**: [`TickDriver`](../../crates/voxel-work/src/tick.rs) is the production consumer — it plans each tick, refuses a rewind, is idempotent within a tick, and digests the plan (`TickReceipt`). The client's frame loop now reads `TickDriver::due_at` for its stage decisions instead of restating the ladder. Demonstrated with real units by `cargo run -p voxel-work --example tick-driver`. |
| The process runtime | `voxel-shapers/src/process.rs` (`ProcessScheduler::step`, `body_of`, `owed_body`) | a caller, plus the **eight bodies** §2–§5 below name |
| The deterministic-tick claim | `C-96` (the tick is a *schedule*, never state) | a test that a ticked world and a rebuilt world produce one digest, whatever the worker count |

**No external source is needed here.** This is this project's own architecture
(`IDEA.md`: *"real-world derived systems that run through a cadence ticker"*).
The cadence ladder itself is closed at four rungs (`stages.rs::Cadence::ALL`).

**Landed since, and measured.** The process runtime now has **two** callers. It carries the
**region form** (`ProcessRequest::unit` and `::region`); `Fields`, `Erosion`, `SoilTransport`
and `Transition` have real region bodies; and `Ecology` has the two capacity laws §5 named.
The roster therefore runs **14 of 16** stages from the first tick
(`cargo run -p voxel-shapers --example process-tick` → `tick 0: 16 due, 14 ran, 2 refused`),
where it ran **1** before the region form and **11** before this pass. The two refusals are
`Caves` (its occupancy field is still not one of the nineteen) and `Earthworks` (the record,
by design). **`Transition` runs**: the `Ecology` dependency it was blocked on now answers.

**The second caller is the field plane itself.** `process::field_at` is the production seam:
`chunk_fields::law_of` routes nine declared-but-unproduced fields (`Tectonics`,
`ErosionResistance`, `Temperature`, `Groundwater`, `Pressure`, `Precipitation`, `Wind`,
`CarryingCapacity`, `SpeciesSuitability`) through the stage the field's own `outputs`
declaration names. The client's frame loop also steps the roster once per tick at the eye's
own unit (`App::drive_processes`). `LEDGER.md` **C-175** is the record.

## 2. Erosion — the two laws that are genuinely absent

`erosion.rs` already implements **thermal** (Musgrave, Kolb & Mace 1989, exactly
volume-conserving) and the **droplet** method (Hansson 2017, declared
approximate). Neither reaches the terrain. The literature supplies the two
*landscape-scale* laws that do the work at the cadence / region scale:

### 2.1 Fluvial incision — the stream-power law (the missing terrain term)

- **Law.** `E = K · A^m · S^n` — erosion rate rises with drainage area `A` and
  channel slope `S`. Unit stream power `ω = ρ g (Q/b) S`, and `Ė = k_ε · ω`.
- **Sources.** Whipple, K.X. & Tucker, G.E. *Dynamics of the stream-power river
  incision model.* JGR 104(B8), 1999. Lague, D. *The stream power river incision
  model: evidence, theory and beyond.* ESPL 39, 2014. Derivations: GeomorphOnline,
  *The stream-power law* (CC BY-SA 4.0).
- **Exponent note from the literature.** The observed downslope decrease of
  channel slope with drainage area is reproduced for `m/n ≈ 0.5` (Lague 2014);
  `m` and `n` are **declared dials**, not invented constants.
- **Wiring point here.** **Landed**: `erosion::stream_power_rate` and
  `erosion::hillslope_transport` (with `erosion::CRITICAL_SLOPE`) are the sourced laws,
  and `process::erosion_body` applies the stream-power rate over the unit's own
  drainage area, emitting `FieldId::ErosionResistance`. It is a **region** body and
  refuses a site request by name. The coefficients `K`, `m`, `n` are declared dials.
  Cadence: `Cadence::Sediment` (×20). The terrain term itself is still unwired — the
  body produces a reading; nothing yet lowers `terrain::elevation_m`.

### 2.2 Hillslope transport — nonlinear diffusion (the missing `SoilTransport` body)

- **Law.** Linear diffusion `q = −K ∇z` (`∂z/∂t = K ∇²z`) is the simple case;
  Roering et al. show transport is **nonlinear** near a critical slope `S_c`:
  `q = K S / (1 − (S/S_c)²)`.
- **Sources.** Roering, J.J., Kirchner, J.W. & Dietrich, W.E. *Evidence for
  nonlinear, diffusive sediment transport on hillslopes and implications for
  landscape morphology.* WRR 35(3), 1999. Composes with the already-cited
  Heimsath et al. 1997 soil-production function (`substrate.rs`).
- **Wiring point here.** **Landed**: `process::soil_transport_body` is the baseline plus the
  move, and `process::soil_transport_delta_m` is the **discrete operator stated rather than
  implied** — a region gives the site one *pairwise* slope per neighbour, the flux is directed
  toward the lower of the two, so a pair exchanges equal and opposite material and a closed
  region conserves exactly. It reads `Terrain, Substrate, Hydrology, Erosion` and moves loose
  material off the baseline. Cadence: `Cadence::Sediment`. Its step is the declared dial
  `SOIL_TRANSPORT_STEP_S`.
- **Thermal talus** (the angle-of-repose relaxation, `erosion::thermal`) is the
  third leg and already exists; its *dial* (the talus angle) is a material
  property and therefore the user's.

## 3. Karst dissolution — the cave occupancy field

- **Law.** Dissolution `C` as a function of flow, `CO₂`/water aggressiveness,
  and residence time; the cave class is a **closed vocabulary** `Open / Flooded
  / Rock`.
- **Sources.** Ford, D. & Williams, P. *Karst Hydrogeology and Geomorphology.*
  Wiley 2007 (dissolution kinetics, chemical equilibria, physical flow laws).
  Nie, X. et al. *Realistic Simulation of Dissolution Process on Rock Surface.*
  Algorithms 17(10):466, 2024.
- **Wiring point here.** `process.rs::owed_body(Caves)` names the gap exactly:
  *"the cave answer (open/flooded/rock) is not one of the nineteen declared
  fields; cave occupancy's field is owed by name."* **The threshold and the water
  table are world-shaping and the user's** (C-174); the *field* is the missing
  system, not the dial.

## 4. Delta and transition — law-plus-state and region blend

### 4.1 Delta — deposition and avulsion (law **plus state**)

- **Law.** Deposition where velocity drops below the Hjulström/Shields threshold;
  avulsion when the channel bed builds past its neighbour.
- **Sources.** Gilbert, G.K. 1885; Hjulström 1935; Shields 1936 (all already in
  `SHAPERS.md`); Slingerland, R. & Smith, N.D. *River avulsions and their
  deposits.* 2004.
- **Wiring point here.** **Landed**: `delta_body` in `process.rs` admits the shaper's
  own deposition law (`velocity_m_s` -> `transport_capacity` -> `deposition_m`) over
  the substrate baseline, so the module is called and produces a reading. **Still
  owed and named:** the long-term state (`DeltaState`: accumulation and avulsion) has
  no home yet — a per-tick reading is not an accumulation, and a counter that
  remembered would be a second simulation.

### 4.2 Transition — the blend law (partition of unity)

- **Law.** Weights that sum to one, smooth across the boundary: inverse-distance
  weighting (Shepard 1968 — already cited) or **normalized sparse convolution
  over a jittered grid**, which avoids the grid-aligned creases a naive lerp
  produces.
- **Sources.** Shepard, D. 1968 (already in `SHAPERS.md`); NoisePosti.ng,
  *Fast Biome Blending, Without Squareness* (2021) — jittered triangular point
  distribution + normalized sparse convolution, weights always sum to one.
- **Wiring point here.** **Landed**: `ProcessRequest::region` is the region form
  (neighbours by distance, borrowed from the caller), and `transition_body` feeds it
  to the shaper's own `shepard_weights` + `blend_class` — so a boundary picks the
  dominant domain and a tie refuses. A request with no region refuses by name rather
  than guessing, which is the honest reading of *"a site alone cannot evaluate it"*.

## 5. Ecology and fields — the remaining owed bodies

- **Ecology capacities.** **Landed**: `ecology::carrying_capacity` is **Lieth's Miami model**
  (Lieth 1975 — two productivity curves, the minimum is the limiting one) limited by the
  substrate's own soil depth, and `ecology::species_suitability` is **Holdridge's humidity
  ratio** (Holdridge 1947, `PET = 58.93·T_bio`) with a *declared* shape over it (`min(r, 1/r)`)
  that is named as this tree's reading rather than attributed to a paper.
  `process::ecology_body` reads both and refuses `MissingSuitability` / `NegativeCarryingCapacity`
  by name rather than producing a zero that would read as a barren province. Leemans, R.
  *Holdridge life zone classification* (IIASA WP-90-008, 1990) remains the parameterisation
  reference if the zones are ever used directly.
- **Fields ladder.** F4's 20-level ladder is arithmetic; `owed_body(Fields)`
  says it *"has no world sampler at a site"* — the missing system is the sampler,
  not a paper (Mandelbrot fBm / Shepard 1968 already supply the primitives).
- **Earthworks** is deliberately bodyless: it is the **record**, not a generator
  (F10). Not a missing system — a refused one, by design.

## 6. What is the user's, and is therefore not decided here

Everything in §1–§5 that changes *what the world looks like* is the user's call,
named rather than assumed:

- the `cave_threshold` and whether the water table follows the terrain (C-174);
- `erosion_droplets_per_column` and the `K`, `m`, `n` of the stream-power law;
- the talus angle per material (a material property);
- whether `delta`/`transition` run as region processes at all, and their home state.

**What remains owed after C-175** (and is therefore still open here): `Caves`' occupancy field
is not one of the nineteen; `Delta`'s long-term state (`DeltaState`: accumulation and avulsion)
has no home; and `erosion` still does not lower `terrain::elevation_m` — the body produces a
reading. Those three are the whole of the list.

The **systems** (laws, sources, wiring points) are what this file supplies; the
**dials** stay with the user.
