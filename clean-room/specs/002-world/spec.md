# Spec 002 — U2 World model (identity, coordinates, placement, materials)

- **Unified system:** `U2 — The world model and its vocabulary` (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2)
- **Derived from (documents):** `docs/DESIGN-SPEC.md` §1 (the world model: 1 m voxel, 32³ chunk,
  4×4×4 subvoxel, 256× addressed detail, the 20-level ladder, §1.3 determinism), §2 F1–F3, §5 scale
  contract; `docs/MODS-MATERIALS.md` §0 (the earth model, never the Minecraft model);
  `LEDGER.md` C-10 (the ladder's kilometre correction) and C-39 (a reflectance is impossible to
  carry without a source).
- **Legacy witness (evidence only):** `crates/voxel-model/*`. Read for *contract and behaviour*,
  never as source (Constitution P7).
- **Status:** `in progress` — identity, coordinates, placement relations, materials, the shape laws,
  the shaper taxonomy and the chain are built and measured; the civilian vocabulary (the Axis/Laya
  ladders) remains owed (§7).

## 1. Why this system exists

Every other system speaks in this one's nouns. A pixel is shaded for a surface that sits at a
*coordinate*; terrain is generated for a *chunk*; an edit invalidates the *chunks a face reaches*;
a mod names a *material*. If these nouns live in several crates, they drift — two copies of "which
chunks does this voxel touch" is two answers to one question, and the one that is wrong is found in
the world. So the world model is one system with **one owner per concern**: one immutable identity
(`seed`, declared period), one coordinate algebra, one material registry, and one place that knows a
world's identity is *not* its residency cache.

It is slice 2 because U1's surface, U3's shapers and U4's streamer all consume it, and because
determinism (P3) is a property of *this* system before it is a property of any of them.

## 2. User scenarios

- **US1 (P1) — coordinates are right where the numbers are negative.** *Value:* the world extends
  both ways from the origin without a seam. *Independent test:* round-trip every voxel in a range
  spanning negatives through `(chunk, within-chunk)` and back. *Given* a signed 32-bit coordinate,
  *When* it is split into its chunk and its offset, *Then* recomposing gives the original — including
  `x = -1` landing in chunk `-1`, never chunk `0`.
- **US2 (P1) — the world is a pure function of `(seed, tick, declared period)`.** *Value:* two runs
  agree, so a report is reproducible. *Independent test:* `truth_digest`. *Given* one identity and
  tick, *When* truth is digested twice, *Then* the digests are equal; *Then* a moved anchor and any
  amount of residency change leave it **unchanged**, while a changed tick changes it.
- **US3 (P1) — a cache cannot masquerade as truth.** *Value:* a performance knob never rewrites the
  world. *Independent test:* residency digested under three different insertion orders. *Then* the
  three digests are identical.
- **US4 (P1) — a material's mass is exact and its colour is sourced.** *Value:* a claim about a
  material can be audited. *Independent test:* an unadmitted material is refused by name, and a
  density that is not a positive exact rational is refused by name; a material with no admitted
  reflectance admits but derives **no** colour.

## 3. Functional requirements

- **FR-001** The voxel coordinate is **signed 32-bit**, `z` is **up**, and the chunk is **32³**
  voxels; a coordinate maps to its chunk by **Euclidean** division (so negative coordinates land in
  negative chunks) and to a within-chunk offset in `0..32`.
- **FR-002** `VoxelPos → (ChunkKey, WithinChunk) → VoxelPos` **must** round-trip exactly for every
  signed 32-bit coordinate.
- **FR-003** The **subvoxel** index is `0..4` per axis and the **detail** index is `0..256` per axis;
  either out of range **must** be refused by name.
- **FR-004** The regional ladder is **20 levels**, level `L` spanning `32 · 2^L` blocks; a region key
  at a level **must** be derived by Euclidean division in `i64` and **must** refuse a level past the
  ladder. Level 19 **must** equal `16 777 216` blocks, i.e. **16 777.216 km**, and the "16.8 km"
  figure **must not** reproduce (`LEDGER.md` C-10).
- **FR-005** World truth **must** be a pure function of `(seed, declared period, tick)`. Its digest
  **must** exclude the anchor and the residency set.
- **FR-006** Residency is a cache. Building it in any insertion order **must** produce one digest.
- **FR-007** A **closed** `Domain` enum names the deterministic sub-streams; `stage_seed(domain, pos)`
  **must** be pure, and two different `(domain, pos)` pairs **must** differ on the fixture.
- **FR-008** Materials are admitted **once** into **one** registry. A duplicate id **must** be
  refused; a lookup for an unadmitted id **must** be refused, naming the id.
- **FR-009** Density is an **exact rational** in g/cm³. A zero denominator or a non-positive value
  **must** be refused by name.
- **FR-010** A `Reflectance` **must** carry a source; an empty source or a channel outside `[0, 1]`
  **must** be refused by name. A material with `reflectance == None` is admitted and derives no
  colour — named absence, never black.
- **FR-011** `chunks_reached_by(pos)` **must** return the containing chunk plus one neighbour per
  chunk-boundary face the coordinate sits on (at most three), and **never** a diagonal neighbour.
- **FR-012** Nothing in this system **must** import, `use`, or path-depend on any crate under
  `crates/`.

## 4. Measurable success criteria

- **SC-001** Round-trip is exact over every coordinate in `[-70, 70]³`. *Decided by:* the test
  `coordinates_round_trip_across_the_origin`, which prints the count it checked.
- **SC-002** Same `(seed, period, tick)` → equal `truth_digest`; a changed tick → different; a moved
  anchor and a populated residency → **equal**. *Decided by:* `truth_is_a_function_of_seed_tick_and_period`.
- **SC-003** Three insertion orders → one `residency_digest`. *Decided by:*
  `the_cache_digest_does_not_depend_on_insertion_order`.
- **SC-004** An unadmitted material and a bad density are refused by name. *Decided by:*
  `an_unadmitted_material_is_refused_by_name`, `a_density_that_is_not_positive_is_refused`.
- **SC-005** Ladder level 19 = 16 777 216 blocks = 16 777.216 km, and the "16.8 km" figure does not
  reproduce. *Decided by:* `the_ladder_arithmetic_is_the_corrected_one`.
- **SC-006** `cargo test` (offline) passes, `cargo clippy --all-targets -- -D warnings` is clean, and
  `cargo run` prints the U2 digests and exits 0.

## 5. Key entities

- **`VoxelPos`** — signed 32-bit `{x, y, z}`, `z` up. **`WithinChunk`** — `{x, y, z}` in `0..32`.
- **`ChunkKey`** — the chunk's signed integer key; the unit of ownership and of streaming.
- **`SubvoxelIndex`** — `{i, j, k}` in `0..4`; the stored shape/composition lattice.
- **`DetailIndex`** — `{i, j, k}` in `0..256`; the addressed appearance lattice (never stored dense).
- **`RegionKey`** and the ladder function (`ladder_extent_blocks`) — the 20-level ladder and a
  region key at a level.
- **`WorldIdentity`** — `{seed, period: DeclaredPeriod}`; immutable. **`Tick`** — the world's clock.
- **`Domain`** — the closed set of deterministic sub-streams.
- **`World`** — the single owner of one identity, one tick, one anchor and one residency set, with
  **truth** and **cache** kept as separate concerns.
- **`Profile`** — the shape laws: the silhouette each shape puts in a voxel, and the exact
  **normal** that shape displaces with.
- **`ShaperKind` / `Level` / `StoredLattice` / `apply` / `ShaperDeclaration` / `ShaperSet`** — the
  two-kinds × two-levels contract, refusing a shaper that claims a level and declares nothing.
- **`Chain` / `Event` / `EventKind` / `Ticket` / `Namespace`** — the append-only digest-linked
  ledger, its tickets, and the exact wire form.
- **`MaterialId` / `Material` / `Density` / `Reflectance` / `MaterialRegistry`** — what is placed.
- **`Digest`** — the project's one content identity (`SHA-256`).
- **`Refusal` / `RefusalSet`** — the named-refusal vocabulary; `RefusalSet` collects **all**.

## 6. Edge cases and refusals

| Input | Disposition |
|---|---|
| any signed 32-bit coordinate | valid; chunk via Euclidean division (FR-001) |
| subvoxel axis outside `0..4` | refuse `subvoxel_out_of_range` |
| detail axis outside `0..256` | refuse `detail_out_of_range` |
| ladder level ≥ 20 | refuse `ladder_out_of_range` |
| a moved anchor / changed residency | **not** a truth change; the world digest is unchanged (FR-005/006) |
| lookup of an unadmitted `MaterialId` | refuse `unadmitted_material`, naming the id |
| duplicate material id | refuse `duplicate_material` |
| density denominator `0` or value ≤ 0 | refuse `non_positive_density` |
| reflectance with no source | refuse `no_provenance` |
| reflectance channel outside `[0, 1]` | refuse `reflectance_out_of_range` |
| material with no reflectance | admitted; derives **no** colour (FR-010) |

## 7. Assumptions and open questions, and what is owed inside U2

- **Built since the first U2 pass (2026-10-11):** the shape laws (`Profile`), the two-kinds ×
  two-levels `ShaperSet` contract, and the append-only `Chain` with its tickets and exact wire form
  (`tasks.md` T013–T015).
- **Still owed, and named rather than stubbed:** the **civilian/government** (Axis/Laya) vocabulary
  (`tasks.md` T016). This pass chose depth on T013–T015 over breadth across all four.
- The material starter set is **small and honestly labelled**; the mods-derived roster
  (`docs/MODS-MATERIALS.md`) is a later content pass. An unadmitted material stays a refusal.
- Whether the world ages with the clock (a naive tick move reshuffles it) is a **behavioural**
  question for U3/U4; U2 fixes only that truth is a pure function of the tick.
