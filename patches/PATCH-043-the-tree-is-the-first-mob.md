---
patch_id: PATCH-043
title: The tree is the first mob — Gaia as its producer, the block tree deleted, the body drawn at the lattice floor, and a build that cannot pass unverified
status: approved
operation: plan
target: shploingus/gaia + shploingus/being + voxel-shapers/flora + shploingus/build
base_ref: shploingus-clean@main
base_digest: sha256:c3f884806091f57b141ee3c15c459de6184b2307fcc472e2130eeb0012b56062
dependencies: [PATCH-042]
source: user-directed-conversation
created_at: 2026-10-21
approval: "user-approved 2026-10-21 — the first mob, the removal, and the gate, applied as PATCH-042's purpose requires"
live_mutation: false
---

# Change

**The record was the deliverable**, as it was for `PATCH-042`, and it is now **approved**: nothing
under `crates/` moves until the slice below is applied, and the approval's purpose is
`PATCH-042`'s — verification that a readable, cited, pinned package can reconstruct a feature — with
the tree as the first feature that has to survive it. A tree is a good first subject precisely
because it is **not** code: if the loop can reconstruct a living thing with a mass, a signature and a
drawn body from cited claims, it can reconstruct a module.

**And the tree is the matrix's first row, not only its first subject.** `PATCH-042`'s calibration
matrix is keyed by *(segment, axis)*, and this patch is what fills `segment = world` for a living
thing: the tree's row cites the patch that declared it, the being record that names it, the axis
readings its mass and signature earn, the star and the tier the verification gives it, and the recipe
line that rebuilds its body from the lattice. A tree that cannot produce that row is a tree the matrix
cannot hand anyone to fix — which is the whole point of building the matrix.

Four rulings, in the user's own words, are the whole of this patch's authority:

> *"okay now since you've done this out first mob is going to be the tree and to remove the old
> block based tree"*

> *"trees would require gaia"*

> *"you'll need to get scientific documents on tree growth for it's subvoxel shaping and growth in
> the 1/8th scale world systems"*

> *"add to the patch the strict requirement that when the project is ran or tested, it first checks
> it's current build hash against a securely encrypted one from the civilian system, and fails the
> cargo build if the changes aren't verified through the game's systems, providing git repo tickets
> for all the missing;/failed/refused items"*

and the follow-up that closes the loop the gate opens:

> *"you are going to have to fix those events that would be duplicated"*

So this record does four things: it **makes the tree a being** (the first mob), it **gives that
being a producer** (Gaia), it **deletes the old block-based tree**, and it **makes the build fail
unless the game's own system has signed the tree that produced it**.

**Landed on disk while this record waits, and it is only the substrate.** `crates/axis/being`
(`voxel-axis-being`) now exists — the seven faces including `Ghost`, `Mobility`, `LifeStage`,
`State`, `Needs`, `SpiritualSignature`, `CreationRecord`, the four named refusals and the
creation-precedes-reference check — the workspace declares it, and the census reads **17** crates with
its row's witness present. That crate is the tree's *being*, not its *producer*: `flora.rs` is
untouched, no tree is a `Being` yet, the block tree still stands, and Gaia does not exist. The removal
may not land before the producer (**RM4**: no build in which the forest is missing), so the next unit
is Gaia plus the removal in one commit, exactly as ordered below.

# The finding, measured

**The tree is a voxel property today, and the project's own doctrine already forbids that.**
`crates/voxel-shapers/src/flora.rs` (516 lines, `sha256 a9b523d4…02b2a6`, 9 tests) is a deterministic
lattice that answers *"what material is at this voxel"*: `Tree::voxel_at` (`flora.rs:106`) returns
`WOOD` for the column `dx == 0 && dz == 0` — **one whole voxel, 8 real metres across** — and
`tree_material_at` (`flora.rs:306`) is called from the world's single material law:

```
chunk_fields.rs:700   if let Some(material) = flora::tree_material_at(spec, x, y, z, surface) {
chunk_fields.rs:701       return Ok(material);
chunk_fields.rs:702   }
```

`docs/GAIA.md` §7 (`0141`, 228 lines, `sha256 7c023225…2c029cf`) states the rule this breaks, in its
own words:

> *"a vine is an **entity on the same governed actor pipeline as a citizen**: it has a position, a
> target selection, a movement delta, and an evidence record. It is **not a voxel property**, so it
> moves through the world rather than being part of it."*

A tree in `flora.rs` is exactly the thing that sentence forbids: it has no identity, no position of
its own, no record, and no mass — it is a *material rule*, and the world cannot tell one tree from
another or say how old either is.

**And the scale arithmetic says the block tree was never a tree.** `scale.rs:31` declares
`REAL_METRES_PER_VOXEL = 8.0`; `coord.rs:38` declares `SUBVOXEL_SIDE = 4`, so the finest plane this
engine can draw on is **2 real metres**. The old rule's own numbers, converted:

| the old tree's field | engine | real | a real tree of that height |
|---|---|---|---|
| `trunk_height` conifer `7 + (h>>40)%6` | 7–12 voxels | **56–96 m** | redwood / Douglas fir class |
| `trunk_height` broadleaf `4 + (h>>40)%4` | 4–7 voxels | **32–56 m** | mature beech / oak |
| trunk cross-section (`dx==0 && dz==0`) | 1 voxel | **8 m** | 0.8–1.5 m — **5–10× too wide** |
| `canopy_radius_m` conifer `2.6` | 2.6 voxels | **20.8 m** | 4–7 m — **3–5× too wide** |
| `canopy_radius_m` broadleaf `2.4 … 3.2` | 2.4–3.2 | **19.2–25.6 m** | 3–6 m |
| the canopy as a solid | a filled sphere/cone | ~37 700 m³ at r = 20.8 m | **1.5–4.7 % of a crown** is leaf + wood; **95.3–98.5 % is void** (Zhu et al. 2023, five trees, §Evidence) |

So the old canopy is a solid ball twenty to sixty times the leaf-and-wood volume a real crown holds,
and its trunk is a whole 8 m column because a voxel was the only unit available. **That is the
argument for the user's ruling, in numbers:** a tree is smaller than the lattice it stands in, so it
cannot be made of that lattice without being wrong by an order of magnitude. It has to be a being
whose *mass* is exact and whose *body* is a drawing at the floor.

**And the stand cannot hold a forest either.** `TREE_CELL_M = 8` is 8 engine metres, so one cell is
`8 × 8` voxels = **64 × 64 real metres = 0.41 ha**, and a forest formation keeps every candidate
(`trees_per_formation` returns `1.0` for five forest biomes). The lattice's ceiling is therefore
**2.4 stems per hectare**, while its own canopy is 41.6 m across — the crowns of two neighbouring
trees would sit 22 m apart. `docs/LEDGER.md:11617` records the census the rule actually produces:
**1 014 conifer + 1 098 broadleaf within ±512 m**, which is ~2 013 trees/km² over 1.05 km² ≈
**20 stems/ha**. By its own geometry the old rule describes an orchard of giants, not a stand. A
closed stand is a *crowd* — many trees inside one 8 m voxel — which is why the tree layer needs a
population (a `Stand`) as well as a drawn being, and why `GAIA.md` §8's own line is load-bearing:
*"Level of detail carries distant populations below the ceiling."*

**Gaia does not exist.** `grep -rin gaia --include=*.rs crates` → **one** hit, a comment
(`weather.rs:277`, *"Gaia's"*). There is no `gaia.rs`, no mob, no plant, no being:

* `grep -rn "struct Being\|SpiritualSignature" --include=*.rs crates` → **0**;
* `crates/planet-affect/src/` exists and holds **0 files** — the affect crate is a directory, not a
  port;
* `voxel_model::civilian/` (11 files, 5 471 lines) has **0** callers (measured in `PATCH-042`), and
  its own header records the ruling that keeps physical mobs *out* of the civilian port: *"civilians
  need the entire process (just not the physical mobs, probably useful for the laya observer and
  sound engine though)"* (2026-10-02). **A mob was never in scope for that port**, so the first mob
  is not a missing piece of the civilians — it is a layer that was never started.

**The drawing path already accepts a being's body, and nothing else has to change to draw one.**
`scene.rs:294` iterates *any* `&Mesh` and pushes one origin per mesh before its quads
(`scene.rs:298`), so a body is one more mesh with its own origin. `main.rs:2402` and `main.rs:2410`
pass exactly one iterator — `streamer.meshes()` — and that is the single call site to extend.

**The build gate exists as a panic and not as a verification.** `shploingus/build.rs` (584 lines,
`sha256 c9ecf2ce…dd0d54`) already fails a build by name when the tree is not conformant
(`build.rs:574`: *"the tree is not conformant: N rejection(s), M unchecked cell(s) …"*), and it
already emits `cargo:rerun-if-changed` per source (`build.rs:38–72`). `voxel-package/src/signing.rs`
(371 lines, `sha256 26cee81b…340635`) is the signing **seam** — `KeyId`, `Signature`,
`Verifier::verify`, and `NoSigningBackend`, which refuses every package. **No backend crate exists**:
`census.rs:1194` asserts `voxel-envelope` is *not* a crate, and `features.rs:236` names
`crates/voxel-envelope/src/lib.rs` as the file that feature needs. So the gate the user asked for is
buildable on a seam that is already the right shape, and the crate it needs is already named in two
places and absent in both.

**The duplicate defect has already happened once, in this file, with a measurement.** `filing.rs`'s
`grow` header records it in its own words: *"`chains/workspace.chain` went from **842,609,979 to
1,685,220,970** bytes across a single filing, and `2 × 842,609,979 + 1,012 = 1,685,220,970`"* — every
grow re-rendered the whole chain and appended it again. The fix was to slice the prior prefix after
checking it, and the reader that catches a repeat is `Board::from_chain`'s `duplicates: Vec<Duplicate>`
(`ticket.rs:956`), which keeps **both** tickets and names both positions. A build gate that appends a
refusal on every failing build is the same defect with a smaller number attached, so the gate in §The
build gate is designed around that reader rather than beside it.

# The being: one crate the whole living layer sits on

`crates/axis/being` (`voxel-axis-being`), a member of the folder `PATCH-042` created, depending only
on `voxel-model`. It owns what every face shares and nothing that any one face owns.

| item | what it is | why it is here |
|---|---|---|
| `Being { id, face, body, state, needs, signature, creation, mass }` | the substrate: a record with an id, never a mutable blob | `CIVILIANS.md` §3.1: *"A **being** is a **record with an id**, and its state is a set of **records, not mutable fields**"* |
| `BeingId` = `being_<sha40(creation)>` | identity **is** the creation record's digest | ids come from the record, never a counter (the tree's own rule) |
| `Face` = Civilian · Pilot · God · **Mob** · Plant · Incarnate · **Ghost** | seven rows: the six the doctrine names plus the ghost the user has now declared | `CIVILIANS.md` §3.1's table, verbatim; the ghost is the row the table was missing |
| `Mobility` = Rooted · Creeping · Walking · Flying | a face's declared way of acting on space | the mob row's *"mobility"* becomes a value, so `Rooted` is a mob that acts without walking |
| `LifeStage` = Seed · Sapling · Mature · Veteran · Dead | a declared ladder, not a float | *"growth, a life stage"* (`CIVILIANS.md` §3.1); death is a **stage**, not a deletion |
| `Needs { light, water, nutrient }` | three declared 0…1 measures with their own basis | C1 line 160: needs are the being's own state |
| `SpiritualSignature { angle: u16, radius: u16, bands: [u16; 12] }` | the spectral read: 432-step wheel, 12 bands | `AFFECT.md`: *"the reading is still computed and kept"* even where the screen cannot show it |
| `CreationRecord { tick, parent, mass, provenance }` | what makes a reference legal | `PATCH-042`'s GH4: creation precedes reference |
| `State` = Alive · Retired · Dead | three states; **no** `remove` | *"Nothing can kill a being"* (`CIVILIANS.md:118`); retirement is explicit and keeps the record |

**The ghost moves here, and `PATCH-042` is amended by one line rather than rewritten.** That record
declares `Face` and `SpiritualSignature` inside `voxel-axis-civilians`; a ghost is not a civilian, and
a tree is not one either, so the two declarations belong in this crate and the civilians crate
depends on it. `PATCH-042` §Ghosts keeps its doctrine, its gates GH1–GH5 and its evidence; only the
**declaration site** moves, and the amendment names this patch.

**What this crate refuses, each with a live subject:**

| refusal | the rule |
|---|---|
| `AbsentSubject` | a selection, row or reference naming a being with no creation record (`GH2`) |
| `NoSignature` | a ghost — or any being — drawn with no `SpiritualSignature` (`GH5`) |
| `Destroyed` | any API shaped `remove`/`delete`/`destroy` on a being; the absence suite proves it does not exist (`GH3`) |
| `StageNotDeclared` | a life stage outside the five, by name |
| `Massless` | a creation with no exact `Ratio` mass; a being is matter before it is a character |

# Gaia: nature's god, and the producer of mobs and plants

`crates/voxel-gaia` (`voxel-gaia`), depending on `voxel-model`, `voxel-shapers`, `voxel-mesh` and
`voxel-axis-being`. `docs/GAIA.md` is the authority and it is already written; this crate implements
it rather than re-deciding it. The five rules it must obey, each quoted from that document:

| # | rule | the document's words |
|---|---|---|
| GA1 | **requests advisory, matter binding** | `0131`: *"A refusal to a Gaia request is **advisory, everywhere** … A physical intervention by the player is **always binding**, because it is matter, not permission."* |
| GA2 | **Gaia creates, and conservation is untouched** | `0140`: *"A new entity is the legitimate product of a conserved delta, not an exception to conservation."* `0128`: *"a Gaia action still produces a **conserved delta and a full evidence record**"* |
| GA3 | **disposition is per species and per claim, and hidden** | `0130`/`0147`: *"disposition is fully hidden. Behaviour is the only tell."* |
| GA4 | **its own cadence, reading authoritative state only** | `0150`/`0155`/`0156`: *"the interval between assessments is a **declared function of authoritative state**"* |
| GA5 | **bounds refuse rather than drop** | `0121`/`0143`: *"Beyond it, new births are **refused with a typed refusal**, not silently dropped."* |

**The law does not bind Gaia and the world's does.** `CIVILIANS.md` §3.7, the user twice: *"gaia is
it's own system unbounded by any laws but it's own"*, and *"rot and decay were modelled in the
previous branch- decomposers are a necassary part of the system"*. So Gaia may act on a cadence no
statute sets, and its refusals are advisory; what it may **not** do is invent mass, skip a transition,
or write outside the book. The one sentence that keeps the jailbreak from becoming a second physics:
**Gaia chooses; the book conserves.**

## The tree, as a mob

The user's ruling — *"first mob is going to be the tree"* — is recorded here as a face assignment
rather than argued again. The reference's table makes a plant a being and **not** an agent, and a mob
a being **and** an agent (`CIVILIANS.md` §3.1). A tree is therefore a mob because it *acts*: it
grows, it competes for light, it spreads seed, and Gaia can move it — all four are acts with records.
Its `Mobility` is `Rooted`, which is a value of the face's own property and not a contradiction: the
mob row's mobility is *how the being acts on space*, and a tree acts by occupying and extending it.
The `Plant` face stays for things that are beings and never act.

**A `Stand` is a population, and a being is materialised from it.** This is the piece the arithmetic
in §The finding forces: at 8 m per voxel a closed stand is many trees inside one cell, so the tree
layer has two subjects and they are different kinds:

* `Stand { cell, species_mix, density_stems_ha, mean_dbh_m, mean_height_m, lai, mass_kg, age_y }` — a
  declared aggregate, owned by Gaia, conserved, and **not** a being. It is what a distant forest is.
* `Tree { being, stand, base, dbh_m, height_m, crown_radius_m, age_y }` — a being **materialised** from
  a stand when the player is near enough to see it. Materialising moves mass out of the stand's
  account into the being's; dematerialising moves it back, exactly, and the demo is a `WorldTransition`
  like any other (`world/governed.rs:145`).

Which gives the first mob its first job, and ties this patch to the last one: **a materialised tree
runs the `Verify` task** — the build attestation in §The build gate is produced by a being stepping
the game's own verification, on the game's own cadence, with a record. The tree is not decoration
that happens to be alive; it is the first worker.

## The growth model, and the papers it is built from

The user asked for scientific documents on tree growth for the subvoxel shaping and the 1:8 scale.
The model is four published relations and three declared heuristics, and the heuristics are labelled
because they are not measurements.

**Shape, from allometry.** The height–diameter relation is the empirical one, not the theoretical
exponent: Hulshof, Swenson & Weiser (2015) fit power laws across **293 species** and report slopes of
**0.53 for angiosperms and 0.60 for gymnosperms**, against the elastic-similarity prediction
`H ∝ D^(2/3)` (West, Brown & Enquist 1999, as stated in Price et al. 2022). The tree therefore
declares a **per-species exponent** and uses the measured values, not 2/3:

```
height_m  = k_species · dbh_m^p_species          p = 0.53 broadleaf, 0.60 conifer
crown_r_m = a_species · dbh_m^b_species          the form Evans et al. (2015) fit for eight UK taxa
crown_depth_m = a_h · height_m^b_h               the same paper's crown-depth form
```

Evans et al. (2015) give the adult-tree forms used above — `H = 1.35 + (maxH − 1.35)(1 − e^(−bD))`,
`CR = aD^b`, `CH = aH^b` for DBH > 10 cm — and **their fitted parameters live in a Dryad record
(`doi:10.5061/dryad.2c1s7`) that was not read for this record**. So the parameters are **owed**, the
form is not, and every `SpeciesParameters` row carries `fit_range` and `basis`:

| field | source | status |
|---|---|---|
| `max_height_m`, `max_dbh_m`, `max_age_y` | the species' own published maxima | **owed** (row refused until a citation is pinned) |
| `p_height_dbh` | 0.53 / 0.60 | **read**, Hulshof et al. 2015 |
| `k`, `a`, `b` (height and crown) | Evans et al. 2015 form | **owed** — the Dryad file |
| `wood_density_kg_m3` | the **material registry's own** `bulk_density_kg_m3` (`material.rs:460`) | **exact rational**, already in the tree |
| `form_factor` | stem volume `V = f·(πD²/4)·H` | declared **heuristic** (0.45), labelled |
| `taper_exponent` | `d(h) = D((1 − h/H)/(1 − 1.3/H))^p` | declared **heuristic** (0.6), labelled |

**A fitted equation is refused outside its fit range.** The only biomass equation read for this
record is Chakraborty, Saha & Reif (2016), fitted on **66 dry-site beech with DBH 2–18 cm**
(`AGB = 0.201·D^2.173`, adjusted `R² = 0.94`, D in cm; stem `0.100·D^2.343`, branches + foliage
`0.123·D^1.776`). A 56 m tree has a DBH near 1 m, **seven times past that fit's ceiling**, so using
it there would be an extrapolation dressed as a measurement. Gaia's `mass_of` therefore refuses with
`OutsideFitRange { asked, fitted }` rather than answering, and the mature-tree table is a named
deliverable (§Not done). **This is the one place the first tree can be blocked, and it is blocked
honestly**: the tree may be *drawn* from its shape parameters and may not be *created* until its mass
is either measured or labelled heuristic — and it is the second that lands in this patch, with
`mass_basis: heuristic:form-factor+registry-density` printed wherever the mass is used.

**Growth, from the published curve.** A tree's height and diameter advance by Chapman–Richards:

```
Y(a) = A · (1 − e^(−k·a))^p            the age of maximum increment is ln(p) / k
```

(Fekedulegn, Mac Siurtain & Colbert 1999, which fits exactly this form to Norway spruce top height;
Pienaar & Turnbull 1973 for the forestry form). The tree's clock is the world's own:
`YEAR_TICKS = TICKS_PER_DAY × DAYS_PER_YEAR = 24 000 × 96 = 2 304 000` (`climate.rs:29–34`), so an
80-year beech is **184 320 000 ticks** old and Gaia's per-tree cadence is declared in those units
rather than in frames. **Growth is a mass event, not a scale-up:** each step computes `ΔM` from the
new `(H, D)` through the same allometric mass, and draws that mass from the soil account at the base
— `0128`'s conserved delta — so a tree's growth is auditable in the same ledger as a dig.

**Density, from the stand's own law.** A stand's density moves on the self-thinning line
`w̄ = C·ρ^(−3/2)` (Yoda, Kira, Ogawa & Hozumi 1963) and its size–density index on Reineke's
`N ∝ D_q^(−1.605)` (Reineke 1933), so competition is a **declared relation** rather than a dice roll.
When the relation says a stand is over its line, a tree dies — and death is a story with three
causes in the doctrine (want, violence, age or illness, `Q137`), so Gaia's version is *age and
shade*, and it is recorded as a `Dead` life stage with a cause.

**Rot is required, and it is the mass's way home.** The user: *"rot and decay were modelled in the
previous branch- decomposers are a necassary part of the system"*. A dead tree becomes a
`Stand`-owned mass that decays at a declared rate, returning to the soil account in steps that each
carry a transition. Without that the loop does not close and the world's mass grows without bound —
which is the conservation audit's job to catch, and which is why `0140`'s "conserved delta" is the
first gate rather than a nicety.

## The body: a drawing at the lattice floor, and the mass that is not the drawing

A tree's body is generated, not stored as terrain:

1. **skeleton** — the allometric `(H, D, crown radius, taper)` gives a trunk axis and a crown
   envelope; the crown's shape is a cone (conifer) or a flattened ellipsoid (broadleaf), the two the
   old rule used, kept because they are the right shapes and were never the problem;
2. **occupancy** — the trunk is a tapered column of cells and the crown is a **shell of the envelope
   with a declared gap fraction**, not a filled volume: the measured crown is **95.3–98.5 % void**
   (Zhu et al. 2023) and a voxel-scale gap fraction of **0.903–0.965** on the same trees, so a filled
   sphere would be a lie the size of an order of magnitude;
3. **mesh** — the cells go into a body frame (`VoxelGrid` → `Chunk::from_tree` → `Mesher::build`), and
   the resulting `Mesh` is placed at the being's base. `Mesh::origin_voxel` derives from `mesh.chunk`
   (`mesh.rs:397`), and a being's base is not a chunk corner, so `voxel-mesh` gains **one field**:
   `origin: [i32; 3]` defaulting to `chunk.origin()`, set by `Mesh::placed_at` for a body and never
   read differently for a chunk. `Mesh::digest` (`mesh.rs:430`) does not change, so no chunk's
   identity moves.

**What the resolution can and cannot show, stated so the drawing is not mistaken for the truth:**

| real quantity | the lattice's floor | what the drawing does |
|---|---|---|
| a 0.8 m trunk on a 40 m tree | 2 m | draws **1 subvoxel**; the record keeps 0.8 m |
| a 1.5 m trunk on a 96 m conifer | 2 m | draws **1 subvoxel**; two only above ~2 m DBH |
| crown radius 5–7 m | 2 m | draws a 2–3-cell shell |
| leaf area (LAD up to 1 m²/m³, LAI ≈ 3.2 in the one study read) | not representable | carried as `lai` on the `Stand` and `leaf_mass` on the being |

So the body carries a **`Drawing`** alongside it, and the report prints it:

```rust
Drawing {
    cells: usize,          // drawn subvoxels
    floor_m: f64,          // 2.0 — the lattice's finest plane
    oversize: Ratio,       // drawn trunk width ÷ measured DBH, exactly
    gap_fraction: Ratio,   // what the crown leaves empty, declared
    why: &'static str,     // "the lattice cannot draw a trunk thinner than 2 m"
}
```

and the rule that keeps the two apart: **`Being.mass` is the being's, `Drawing.cells` is the
drawing's, and no audit may read one as the other.** A test plants a trunk whose drawn width is 2.5×
its DBH, asserts `oversize == 5/2`, and asserts the conservation snapshot is unchanged by the
drawing's cell count. That is the same split the ghost rule already makes — the reading computed even
where the screen cannot show it — applied to a trunk.

**Cost, so "in real time" is a number.** A body of ≤ 40³ subvoxels meshes in one `Mesher` pass, and
`PATCH-042`'s walk budget (306 files / 12.1 MB / 105.6 ms) is the same order. The gate records
`tree-materialise-ms` ≤ 8 ms and `tree-dematerialise-ms` ≤ 2 ms, measured on the near-field stand at
the alpha spawn, and prints both.

# The removal: the block tree is deleted, and the forest never blinks

The user's ruling is *"remove the old block based tree"*, and deletion is the point — a tree kept as
terrain beside a tree kept as a being is two trees and one word (`C-73` §73.1).

| # | file | what goes | what is left |
|---|---|---|---|
| D1 | `voxel-shapers/src/chunk_fields.rs:700–702` | the tree branch of `material_of` (and the comment at `695–696` that says *"a tree where one reaches this voxel"*) | above ground is `WATER` below sea level, `AIR` above it — the law's own words change from *"sea or a tree"* to *"sea or air"* |
| D2 | `voxel-shapers/src/flora.rs` | `Tree::voxel_at`, `tree_material_at`, `trees_over_column`, `MAX_TREE_ABOVE_GROUND_M`, `MAX_CANOPY_RADIUS_M`, `Tree::reaches` | **the file is deleted** (516 lines); its lattice moves to `voxel-gaia/src/lattice.rs` as a *spawn* rule with no material answer |
| D3 | `voxel-shapers/src/chunk_fields.rs:1157–1195` | `the_field_plane_grows_trees_above_the_ground` | `the_field_plane_holds_no_wood_above_the_ground`: the same walk over a now-empty band finds **0** wood, 0 leaf, 0 conifer |
| D4 | `voxel-client/examples/face-refined.rs:319–349` | the search for a tree in the chunk grid | the same measurement on a **materialised body** from `voxel-gaia`, so the example still measures subvoxel faces |
| D5 | `voxel-shapers/examples/flora-census.rs` (89 lines) | the whole example | `voxel-gaia/examples/stand-census.rs`: species, density, mean DBH and body cells per stand |
| D6 | `docs/LEDGER.md:11555–11720`, `LEDGER-COMPACT.md:880–884` | the *current* status of these sentences | the history is **not edited**: `LEDGER.md` gains a dated superseding entry that names `159.11` and says the lattice is now a spawn rule and the tree is a being; a superseded sentence is not a deleted one (D6's CLM) |

**One small finding the removal settles for free:** `voxel-shapers/src/lib.rs`'s module table — the
crate's own list of what it is made of — **has no `flora` row**, even though `pub mod flora;` is in
the file. The tree was never in its own crate's declaration, which is the same defect in minature
that this patch is removing: a subject nobody declared. The deletion means the table does not need the
row; it does need the *three* places that describe the forest (`LEDGER.md`, `LEDGER-COMPACT.md` and
the new `voxel-gaia` header) to say the same thing.

**What must be true after, and is tested:**

| # | gate |
|---|---|
| RM1 | **no chunk holds a tree**: over 27 chunks at the alpha spawn, `material_of` returns **0** voxels of `WOOD`, `LEAF` or `CONIFER`, and the count is printed |
| RM2 | **no second tree**: `grep -rn "tree_material_at\|Tree::voxel_at" crates` → 0, and `voxel-shapers` has no `flora` module (`cargo check` names any survivor) |
| RM3 | **the stand is not lost**: `stand-census` on the same ±512 m reports the same **2 112** stems the ledger's `159.11` counted, now as stand density rather than as tree objects |
| RM4 | **one slice**: D1–D2 and Gaia's first `Stand` + materialised body land in the **same** patch, so there is no build in which the forest is missing |
| RM5 | **a materialised tree is drawn**: the body's mesh is non-empty, its origin is the being's base, and the frame's mesh list contains it (`Scene::build` receives `streamer.meshes().chain(gaia.bodies())`) |

# The build gate: an attestation from the game's own system, or no build

The user's requirement, restated as the gate's own contract: **when the project is run or tested, the
first thing checked is the current build's hash, against a value the civilian system produced and
sealed; if the change has not been verified through the game's systems, `cargo build` fails; and every
missing, failed or refused item is filed as a ticket in the repository.**

**What the hash is.** `build_hash = Identity::of_fields("shploingus/build", [source walk, Cargo.lock,
crate list])` over the same patterns `PATCH-042`'s artifact layer walks — deterministic, path-free,
clock-free, and the same number on two machines. It is *not* a git hash: a dirty tree has no commit,
and the base bytes of this record are dirty already.

**What the attestation is.** One record, signed, never encrypted for verification:

```rust
Attestation {
    base: Identity,        // the build the verifier itself was built from
    change: Identity,      // the tree being attested
    sources: u64, bytes: u64,
    chain_head: Identity,  // the record the verification was filed on
    tickets: Vec<TicketId>,// what the verification left open
    signer: KeyId,         // sha256 over the public key, never the key
    signature: Signature,  // over the canonical fields above
}
```

**"Securely encrypted" is answered precisely, because encryption is not verification.** A secret that
decrypts the value is a secret every builder must hold, and a check every builder can run is a check
nobody's forgery resists. So:

* the **signature** is the check — `voxel-package::signing::Verifier::verify` (the seam that already
  exists), against a public key pinned at `conformance/build-key.pub`;
* the **private key is encrypted at rest** and held **outside the tree** by the civilian host — an
  `age`/XChaCha20-Poly1305 sealed file whose passphrase the game process supplies; no key material
  enters the repository, the chain, or a log;
* the attestation file itself is stored as the signed record (an unreadable blob is not a stronger
  attestation, it is one that cannot be audited).

**Two hashes, so a change can never verify itself.** `base` is the hash the verifier was built from;
`change` is the tree it attests. A verifier built from the same tree it is checking proves nothing, so
the gate requires `base` to be a hash that already has an attestation, and refuses `BaseNotAttested`
otherwise. This is also the bootstrap: the **genesis** attestation is produced by the pre-patch game
over the pre-patch hash, then the gate is enabled; from then on each change is verified by the last
attested build, and the record grows one link at a time. There is **no flag**: `R-59.2`'s rule is that
*a signature that can be bypassed by a build flag is not a signature*, and a crate boundary is how the
tree enforces it — a build without `voxel-envelope` gets `NoSigningBackend`, which refuses everything.

**Where it runs, so "run or test" is literally true.** The gate's home is `crates/voxel-model/build.rs`
(206 lines, `sha256 2d7855a5…a857980f`), because **that file is already a gate**: it recomputes the
materials and spectra digests and *"fails the build when they disagree with `materials/digest.txt`"*
(`build.rs:6–8`), and it already emits `cargo:rerun-if-changed`. Every member of the workspace depends
on `voxel-model` **except three** — `voxel-noise`, `voxel-light` and `voxel-ui` — so those three get a
three-line `build.rs` of their own that calls the same `voxel-attest::gate`. `cargo build`, `cargo run`
and `cargo test` therefore all run it, for every package, on every member.

**It has no door, and that is where it must sit in `build.rs`.** The materials gate deliberately has
one: `materials-regeneration` prints a warning and verifies nothing, so the generator can rewrite a
stale table (`build.rs:20–30`). The attestation check is placed **before** that `CARGO_FEATURE_…`
test, because a feature that skipped it would be exactly the flag `R-59.2` forbids — and because the
obvious place to add it (beside the existing digest check) is inside the block that returns early. A
test enables `materials-regeneration` and asserts the build **still** fails without an attestation.

It fails the build the way `build.rs` already does — `panic!` with the named refusals — and the
message says which of the seven states it found: `MissingAttestation`, `StaleAttestation` (`change` is
not the tree on disk), `SignatureInvalid`, `UntrustedKey`, `UnknownSignatureAlgorithm`,
`BaseNotAttested`, or `NoSigningBackend`.

**The tickets, and the duplicate fix the user asked for.** Every failing item is filed **once**, as a
chain event, and the ticket is the board's derivation of it — never a second list beside the record
(H-8.1):

1. **identity before append.** The gate computes `objective = "build | <gate> | <path> | <expected>"`
   and asks `Board::from_chain` whether a **live** ticket already carries that objective. If it does,
   the gate cites the existing id and **appends nothing**.
2. **the reader is the check.** `Board::duplicates()` already reports an objective filed at two
   positions (`ticket.rs:956`), keeps both tickets, and hides neither; the gate prints that list in
   its failure, so a duplicate that ever slips in is named rather than silently added to.
3. **a transition is a different event, and is allowed.** A failure that was closed and is now open
   again is a `TicketTransition`, not a re-filing; that is the one case where a second event about
   the same objective is correct, and it is a state change rather than a repeat.
4. **the repair path is not automatic.** `filing::repair` (`filing.rs:383`) is the only thing that may
   compact a chain; it refuses when nothing is dropped, refuses if the folded count falls, and
   refuses if the tickets move, id for id. The gate never calls it — a build that repaired its own
   record would be a build that edited its own evidence.
5. **the measured precedent is the test.** `grow`'s own header records a doubling from
   **842,609,979 to 1,685,220,970 bytes** in one filing; the gate's test re-enacts the shape at
   small scale — fail twice with the same bytes, assert the chain grows by **one** event, and assert
   the second run's report cites the first run's ticket.

Tickets are files in the repository as well as events: `tickets/<ID>.md`, rendered from the board by
`voxel-gaia --bin board`, refusing to overwrite, one per id, so `git status` shows what a refused
build owes. The three categories are distinct and each is named:

* **missing** — a thing the substrate requires and the tree lacks (`NoSigningBackend`, an owed
  `SpeciesParameters` row, a `Face` with no producer);
* **failed** — a gate that ran and refused (conformance's rejected cells, `PATCH-042`'s A1–A14, this
  record's T1–T14);
* **refused** — a `RefusalRecorded` event on the chain with no ticket citing it.

**The one new dependency, named.** `voxel-envelope` implements `Signer`/`Verifier` over **Ed25519**
(`ed25519-dalek`, one crate, pinned) because a hand-rolled signature is worse than a dependency and
`R-59.2` requires a real backend behind the seam. `voxel-attest` is the small crate the build scripts
call — it owns the walk, the canonical payload and the `panic!`, and depends on `voxel-envelope`, so a
build *cannot* check a signature without the backend (the crate boundary is the enforcement).
`ed25519-dalek` is the only new third-party dependency, and it makes `features.rs:236`'s row
(`SignatureVerification => "crates/voxel-envelope/src/lib.rs"`) described rather than owed.
`census.rs:1194`'s assertion that the crate does **not** exist and `census.rs:2361`'s test that
`Crate::parse("voxel-envelope")` fails are both inverted by this patch.

The count moves **26 → 29**: `voxel-axis-being`, `voxel-gaia`, `voxel-envelope`. `voxel-attest` is a
build-dependency-only crate and is a member too, which makes **30** — and the count is stated here
rather than discovered by the suite, because `CRATE_COUNT` is a literal that `PATCH-042` already
moves 16 → 26 and this record moves again. The three being/gaia/envelope crates and the attest crate
land in one slice; there is no ordering in which a signature is checked by a crate that cannot build.

# What this changes in PATCH-042

* `dependencies` do not move; `PATCH-043` depends on it, not the reverse.
* `Face` and `SpiritualSignature` are declared by `voxel-axis-being` (this record), and
  `voxel-axis-civilians` depends on that crate. `PATCH-042` §Ghosts' doctrine, GH1–GH5 and evidence
  stand unchanged; only the declaration site moves, and the ghost is the seventh face because this
  record is where a ghost and a tree first needed the same substrate.
* The census count `PATCH-042` sets to 26 is **30** after this record (`voxel-axis-being`,
  `voxel-gaia`, `voxel-envelope`, `voxel-attest`), and the `Exercised` axis `PATCH-042` adds reads the
  first two from their first commit — `voxel-gaia` is `active` because `voxel-client` calls it, which
  is the axis's first `active` row with a call site rather than a test.

# Acceptance

Every gate is a command and a figure; every guard is proven to bite by perturbing its own subject.

| # | gate | stated as | bite |
|---|---|---|---|
| T1 | a tree is a being | `Tree` carries `BeingId`, `Face::Mob`, `Mobility::Rooted`, `LifeStage` and a `CreationRecord`; no tree exists as a material | delete the `Face::Mob` row → does not compile, named |
| T2 | the lattice is a spawn rule and nothing else | `voxel_gaia::lattice::stand_of_cell` returns a `Stand`; no `pub fn` in the crate's signature list returns a `MaterialId` | the signature list is the test: the suite greps `voxel-gaia/src/**/*.rs` for `-> … MaterialId` and asserts **0**, the same shape as the destruction absence suite |
| T3 | a body is drawn from the being | materialise, mesh, assert `Drawing.cells > 0`, `origin == base`, and the frame's mesh list contains it | the body with no cells → `NoBody` |
| T4 | the drawing is not the mass | `oversize` is exact (`5/2` for a 2 m drawn trunk on a 0.8 m DBH); the conservation snapshot is identical with and without the body | the planted trunk |
| T5 | the crown is not solid | drawn crown cells ÷ envelope cells ≤ 0.12, printed as `crown-fill`, against the measured 95.3–98.5 % void | a filled sphere fixture is refused `SolidCrown` |
| T6 | a fitted equation is refused outside its fit | `mass_of` with DBH 120 cm against the 2–18 cm fit → `OutsideFitRange`; the same call at 12 cm answers | the 120 cm call |
| T7 | mass is conserved through growth | `ΔM` drawn from the soil account: the book's total is identical before and after a growth step; the audit prints both | a growth that mints 1 kg → the total moves |
| T8 | materialise/dematerialise is exact | a tree materialised and dematerialised with no growth leaves the stand's mass identical, by `Ratio` | a 1-part-in-10⁶ loss is refused |
| T9 | the age is the world's clock | a mature beech's age is a whole number of `YEAR_TICKS` (2 304 000); a tick cannot advance a stage by less than a season | a 1-tick growth step is refused |
| T10 | Gaia's refusals are advisory and matter is binding | a refused Gaia request does not stop the cadence; a player's dig at the same cell is applied and the stand records it | the two receipts |
| T11 | the bounds refuse | at the entity ceiling a materialisation is refused `EntityCeiling`, not dropped, and the stand's count does not move | the ceiling fixture |
| T12 | death is a stage and rot closes the loop | a `Dead` tree's mass returns to the soil in steps whose sum is its mass, exactly; nothing is deleted | the mass ledger |
| T13 | latency | `tree-materialise-ms` ≤ 8 and `tree-dematerialise-ms` ≤ 2 on the alpha spawn, printed | the printed pair |
| T14 | the stand is the same stand | `stand-census` at ±512 m reports 2 112 stems, matching `LEDGER.md:11617`'s count | the count |
| A1 | no attestation, no build | a clean checkout with an empty `attestations/` fails `cargo build` with `MissingAttestation`, naming the build hash | the empty directory |
| A2 | a change cannot verify itself | an attestation whose `base == change` is refused `BaseNotAttested` | the self-signed fixture |
| A3 | a stale attestation is stale | one byte changed under `crates/` reads `StaleAttestation`, never a pass | the changed byte |
| A4 | the signature is the check | a tampered `tickets` field is `SignatureInvalid`; a forged key is `UntrustedKey`; an unknown algorithm is its own refusal | three fixtures, three names |
| A5 | no flag | no `cfg`, env var or feature turns the gate off; the absence suite finds none, and a build without `voxel-envelope` gets `NoSigningBackend` | the backend-less fixture |
| A6 | every failure is a ticket | one failing gate yields **one** `TicketTransition` event and one `tickets/<ID>.md`; the file's id is the board's id | the filing |
| A7 | a repeat is not a repeat | running the same failing build twice leaves the chain **one** event longer (`chain_events_after - chain_events_before == 1`), and the second run's report cites the first's ticket id | the second run |
| A8 | a duplicate is named, never hidden | a chain with a repeated objective reports it in `Board::duplicates()` with both indices, and the gate prints them | the planted repeat |
| A9 | a repair is never automatic | `build.rs` contains no call to `filing::repair`; the grep is the test | the source |
| A10 | the three categories are distinct | a missing item, a failed gate and a `RefusalRecorded` with no ticket each produce their own ticket kind and their own sentence | three fixtures |
| A11 | the gate runs on test too | `cargo test -p voxel-light` — a crate that depends on neither `voxel-package` nor `shploingus` — still fails `MissingAttestation` | the straggling crate |
| A12 | the regeneration door does not open it | `--features materials-regeneration` on `voxel-model` still fails with no attestation, because the check precedes the feature test | the feature flag |

**Commands and figures.** `cargo test -p voxel-axis-being -p voxel-gaia` → T1–T14 by name, 0 failures;
`cargo test -p voxel-shapers` → RM1–RM3 and the 13 existing `chunk_fields` tests minus one replaced
gate, 0 failures; `cargo test -p voxel-package` → A1–A12 and the existing suites, with `CRATE_COUNT`
at 30; `cargo check -p shploingus` → exit 0 **only** with a valid attestation, and exit 101 with
`MissingAttestation` and the ticket list without one; `cargo run -p shploingus` → the near stand
materialises on approach, `tree-materialise-ms` and `crown-fill` print, and the ApiViewer pane's
`Exercised` row reads `voxel-gaia  active  1 call site`.

# Evidence

* **The bytes this plan measured**, all on the base tree, by `sha256sum`:

| file | lines | sha256 |
|---|---|---|
| `crates/voxel-shapers/src/flora.rs` | 516 | `a9b523d463da8eff24799d3251fbabd820dc3e2c5b4bfe82928f47578d02b2a6` |
| `crates/voxel-shapers/src/chunk_fields.rs` | 1 346 | `911fad0bc92e560d5f627fac21a8903392431af62c86f8b860a14e3c038bfe21` |
| `crates/voxel-shapers/src/scale.rs` | 103 | `75b0cb9a89ffb45cf41052509c2edca7c0bf91e87cfe7d0c20a795ae912a024b` |
| `docs/GAIA.md` | 228 | `7c0232251fd3569ab42af4541f263815a2fae4769e9eb8678cd20b29a2c029cf` |
| `crates/shploingus/build.rs` | 584 | `c9ecf2ce4d242fcffa3f62fd67c39adb8304eba22f4615376036a704fdfd0d54` |
| `crates/voxel-package/src/signing.rs` | 371 | `26cee81bc834b60e5cc0ff3f4c41baa54bf1f03c7580df676ddab37349340635` |
| `crates/voxel-package/src/census.rs` | 2 582 | `8e015e8bd96cf013962dd7508a4a5e7a7b453237fc6dcdc7dff176024fec9e8d` |
| `crates/voxel-client/src/scene.rs` | 1 240 | `3f96c5ec8b64afd7b8fa87309a247f8483bf12f211764f87525021ef7a2d84f8` |
| `crates/shploingus/src/main.rs` | 3 247 | `ff5405384128585123645bc20418692c7c38273b92eee963fb1f5b323a33fd01` |
| `crates/voxel-model/src/coord.rs` | 367 | `c205b842212f8c52b0999b56ee77415afa1d388419b1e5c39b677b35b9a20694` |
| `crates/voxel-mesh/src/mesh.rs` | 1 854 | `8b20bfca9fd058ace16cacaf188910484a72194eedcf4fc1027aa5f12ad3b6b2` |
| `crates/voxel-model/build.rs` | 206 | `2d7855a5fdd9a4c4eea2f459b6c51ddc09bda6d1729244bc161ad1a88579780f` |

* **The counts**: `flora.rs` 9 tests; `chunk_fields.rs` 13; `chains/workspace.chain` 1 532 016 bytes;
  `CRATE_COUNT` 16, `AXIS_COUNT` 6; `TREE_CELL_M` 8, `MAX_TREE_ABOVE_GROUND_M` 16, `CANDIDATE_CHANCE`
  0.85, `DRY_SHARE` 0.35, `MONTANE_SHARE` 0.7; `SUBVOXEL_SIDE` 4, `CHUNK_SIDE` 32,
  `REAL_METRES_PER_VOXEL` 8.0, `TICKS_PER_DAY` 24 000, `DAYS_PER_YEAR` 96, `YEAR_TICKS` 2 304 000.
* **The scale arithmetic** (each a conversion, not an estimate): 1 voxel = 8 m; 1 subvoxel = 2 m;
  conifer trunk 7–12 voxels = 56–96 m; broadleaf 4–7 = 32–56 m; a 56 m conifer's 2.6-voxel crown
  radius = 20.8 m; `TREE_CELL_M` 8 voxels = 64 m per side = 0.41 ha = 2.4 stems/ha; the ledger's
  2 112 stems over 1.05 km² = ~20 stems/ha.
* **The papers**, with what was actually read:

| work | what it supplies | read where |
|---|---|---|
| Shinozaki, Yoda, Hozumi & Kira (1964), *A quantitative analysis of plant form — the pipe model theory I*, Jpn J Ecol 14:97, `doi:10.18960/seitai.14.3_97` | sapwood area ∝ foliage above; quoted as *"a unit amount of leaves is provided with a pipe"* | via Lehnebach et al. 2018, Ann Bot 121:773, `doi:10.1093/aob/mcx194` (the original was not retrieved) |
| Minamino & Tateno (2014), *Tree branching: Leonardo da Vinci's rule versus biomechanical models*, PLOS ONE 9:e93535, `doi:10.1371/journal.pone.0093535` | `A_parent = ΣA_daughter`, i.e. `r²` sums, not `r` | the article |
| West, Brown & Enquist (1999), Nature 400:664, `doi:10.1038/23251`; Price et al. (2022), Plant Physiol 190:1854, `doi:10.1093/plphys/kiac358` | `H ∝ D^(2/3)` as a prediction, not a fitted law | via Price et al. |
| Hulshof, Swenson & Weiser (2015), *Tree height–diameter allometry across the United States*, Ecol Evol 5:1193, `doi:10.1002/ece3.1328` | 293 species; slopes **0.53** angiosperms, **0.60** gymnosperms; Gompertz the best overall fit | the article |
| Evans et al. (2015), *Allometry and growth of eight tree taxa in United Kingdom woodlands*, Sci Data 2:150006, `doi:10.1038/sdata.2015.6`; parameters `doi:10.5061/dryad.2c1s7` | the adult forms `H = 1.35 + (maxH − 1.35)(1 − e^(−bD))`, `CR = aD^b`, `CH = aH^b` | the article; **the parameter file was not read** → owed |
| Chakraborty, Saha & Reif (2016), *Biomass equations for European beech growing on dry sites*, iForest 9:751, `doi:10.3832/ifor1881-009` | `AGB = 0.201 D^2.173`, stem `0.100 D^2.343`, branch+foliage `0.123 D^1.776`, `R² = 0.94`, **DBH 2–18 cm** | the article |
| Fekedulegn, Mac Siurtain & Colbert (1999), *Parameter estimation of nonlinear growth models in forestry*, Silva Fennica 33:327, `doi:10.14214/sf.653`; Pienaar & Turnbull (1973), Forest Science 19:2, `doi:10.1093/forestscience/19.1.2` | Chapman–Richards and von Bertalanffy forms; peak increment at `ln p / k` | the journal pages (the full texts were not retrieved) |
| Yoda, Kira, Ogawa & Hozumi (1963), *Self-thinning in overcrowded pure stands*, J Biol Osaka City Univ 14:107; Reineke (1933), J Agric Res 46:627 | `w̄ = C ρ^(−3/2)`; `N ∝ D_q^(−1.605)` | the conventional forms, verified against secondary sources |
| Zhu et al. (2023), *A reinterpretation of the gap fraction of tree crowns*, Front Plant Sci 14:1109443, `doi:10.3389/fpls.2023.1109443` | five trees: leaf+wood volume **1.5–4.7 %** of crown, void **95.3–98.5 %**; voxel gap fraction 0.903–0.965 | the article and its table |
| Wang et al., *Retrieval of the leaf area density of Magnolia woody canopy*, J Remote Sensing, `doi:10.11834/jrs.20165061` | LAD max **1 m²/m³** at 4 m, LAI **3.20** | the article page |
| Polášek, Hrůša, Beneš & Čadík (2021), *ICTree*, ACM TOG 40(6), `doi:10.1145/3478513.3480519` | *"branching angles, length of branches, and widths are critical for perceived realism"* | the project page |
| Zhang et al. (2017), *Tree branch level of detail models for forest navigation*, CGF 36(8):402, `doi:10.1111/cgf.13088` | branch meshes → *"semi-transparent lines until the tree recedes to a few lines"* | the publication record |
| Runions, Lane & Prusinkiewicz (2007), *Modeling trees with a space colonization algorithm*, `doi:10.2312/NPH/NPH07/063-070` | skeleton generation before geometry | the paper page |
| Lecigne, Delagrange & Messier (2018), *VoxR*, Ann Bot 121:589, `doi:10.1093/aob/mcx095`; Hosoi, Nakai & Omasa (2013), ISPRS Annals II-5/W2:115, `doi:10.5194/isprsannals-II-5-W2-115-2013` | voxel tree analysis at 0.1–1 m; separate **leaf-area density** from **woody volume** | the articles |

* **What the papers do *not* establish, stated because the model leans on them**: there is **no
  published threshold** at which a tree reads as a tree on an 8 m grid, and the 2 m subvoxel floor is
  an engineering consequence of `SUBVOXEL_SIDE = 4`, not a perceptual finding. `ICTree` supports
  keeping branch proportions; it does not license "silhouette is enough". The crown's gap fraction is
  from five trees of five species, and the LAD/LAI numbers from one study.

# Risks

1. **Two hashes and a key is a bootstrapping problem, not a code problem.** If the genesis
   attestation is not produced before the gate is enabled, nothing builds. The plan turns the gate on
   *after* the genesis link exists, and says so in `attestations/README.md`.
2. **A build script that appends to the chain is a writer in a build.** It is fenced to *one* event
   per distinct objective, and the plan's A7 measures it; if it ever proves unstable, the fallback is
   to write the event only under `--tests`' guarantee of a single build script run, never on every
   target.
3. **Removing the block tree changes every screenshot and every face example.** D3–D5 replace the
   subjects rather than deleting the measurements, and `face-refined` keeps measuring subvoxel faces
   on a being's body; a visual review is the user's, not this record's.
4. **The mass model is heuristic for mature trees.** It is labelled, it is refused outside the one
   fit range that was read, and the mature table is owed. The alternative — extrapolating a 2–18 cm
   equation to a 120 cm tree — is exactly the defect this repository convicts.
5. **A tree inside a stand is a crowd, and LOD is now load-bearing.** If materialisation is slow, the
   near field is a few trees, not a forest; T13 bounds it and the fallback is a smaller near radius,
   declared.

# Not done, and named

* **The mature-tree allometry and the Dryad parameter file are not read.** The first tree can be
  created with a `heuristic:` mass basis; it cannot be *evidenced* by one, and the ticket stays open
  until the fitted table is pinned with its fit range.
* **Decomposers are owed.** Rot is a declared decay rate returning mass to the soil; the organisms
  that do it, and their own needs, are `CIVILIANS.md` §3.7's next slice and are not modelled here.
* **No animal or plant face is produced by this patch.** Gaia produces the tree; the mob pipeline's
  other species are rows with no producer, and `Exercised` is what will say so.
* **No violence, no corpse, no court.** A tree's death is age and shade; `Q137`'s three causes and
  the corpse as a material event are `CIVILIANS.md` §3.6's slice, not this one.
* **The MCP surface does not expose Gaia.** `ask_gaia` would be a new `Action` and a matrix ruling;
  the attribution is `PATCH-042`'s owed `ask_von` list, and this patch does not add a row.
* **The private signing key is the user's to create.** The patch ships the seam, the crate, the gate
  and the genesis procedure; it does not generate, store or print a key, and no key material is in
  this record.
* **`clean-room/src/axis.rs`'s copy is untouched**, as `PATCH-042` says; the clean room is a separate
  workspace.

# Rollback

Delete `crates/voxel-gaia` and `crates/axis/being`, remove their two lines from `[workspace] members`,
restore `voxel-shapers/src/flora.rs` from `a9b523d4…02b2a6` and its material branch in
`chunk_fields.rs`, restore the three tests and two examples, drop `Mesh::origin`'s setter, restore `voxel-model/build.rs` to
its 206 measured lines, remove `crates/voxel-envelope`, `crates/voxel-attest`, the three straggler
`build.rs` files, `attestations/` and `tickets/`, restore `census.rs`'s `CRATE_COUNT` to 26 and its
`voxel-envelope` assertion, and revert `main.rs`'s chained mesh list. Re-take the `missing-docs` pins.
Nothing outside those files moved, and the base digest restores the tree that was measured here.
