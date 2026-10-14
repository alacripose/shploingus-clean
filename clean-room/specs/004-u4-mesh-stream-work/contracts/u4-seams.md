# Phase 1 — Interface contracts: U4 meshing, streaming and work

**Feature**: `004-u4-mesh-stream-work` | **Date**: 2026-10-11 | **Spec**: [spec.md](../spec.md) | **Plan**: [plan.md](../plan.md)

The clean room is a **library**: its contracts are the seams its callers use. U4 exposes four, and each
is stated as the *refusal* it must produce as well as the value it must return — a seam that cannot
refuse is not a seam (P2).

## C1 — Derive a chunk's geometry from the world's declared fields

```text
derive(world: &World, chunk: ChunkKey, level: Level, budget: Budget) -> Result<Mesh, Refusal>

Mesh { key: GeometryKey, quads: Vec<Quad>, digest: Digest }

Quad  { layer: Layer::Solid | Layer::Water, height: i32, x: i32, y: i32, width: u32, depth: u32,
        material: MaterialId }         // x and y are world columns; height is the block z (`z` is up)
Level { 0 ..= MAX_LEVEL }          // level L merges up to 2^L columns per rectangle

Refusals:
  - site_alone                a chunk of zero columns is not a request
  - level_off_ladder          a level past the declared render ladder — never clamped
  - unadmitted_material       a lithology class the registry has not admitted, named by class
  - stage_input_absent        a declared field the roster did not produce (U3's)
  - the declaring stage's own refusal, passed through by name (never an empty mesh)
  - geometry_budget_exceeded  the derivation would exceed the declared ceiling
```

**Invalidant — the truth is U3's.** The mesh is derived from `elevation`, `water_level` and `lithology`
through the **one** `field_at` seam. U4 has no terrain function: if the field plane cannot answer, the
mesh is refused, not approximated. The block height of a column is the field's value **rounded to the
nearest metre**, and that rounding rule is the only place a `f64` field becomes geometry — so it is
stated here rather than discovered later.

**Invalidant — a quad is the merge's unit (C-43).** Rectangles are merged greedily over a mask of
equal `(layer, height, material)` cells, so a flat uniform region is **one** quad. Vertices are
`4 × quads`, triangles `2 × quads`, indices `6 × quads`. A *displaced* surface is not planar and cannot
be represented by four corners: that level is owed, and this slice does not pretend otherwise.

## C2 — Keep the geometry coherent with the world it came from

```text
GeometryKey { truth: Digest, chunk: ChunkKey, level: Level }   // truth = U2's truth digest

Resident::get(&world, key) -> Hit(Mesh) | Miss
Resident::insert(key, mesh)
Resident::invalidate(&world) -> usize        // how many stale entries were removed
```

**Contract.** An entry is served only while `key.truth` equals the world's current truth digest. A stale
entry is **rebuilt, never served** (FR-005). Invalidation removes **exactly** the entries whose truth is
no longer current and keeps every entry whose truth is current (FR-006) — the count is returned, so the
caller can print it. U2's anchor and residency cache are **not** inputs (FR-007): a view is not truth,
so moving the anchor changes no digest and drops no entry.

**Refusal**: a chunk outside the declared residency horizon is refused by name
(`geometry_out_of_residency`) and caches nothing. The lookup comes **first**: geometry that is already
resident is served whatever the anchor now says, because the horizon is a limit on *work* (do not build
what will not be kept) and the anchor is not an input to an answer (FR-007).

## C3 — The four horizons are four types (SPEC-5)

```text
RenderHorizon(f64 m) · SimulationHorizon(f64 m) · ResidencyHorizon(f64 m) · FieldHorizon(f64 m)

Horizon::check(render, residency) -> Result<(), Refusal>
  - horizon_mixed   the render horizon is wider than the residency horizon: geometry would be drawn
                    from a set that is never kept
```

**Contract.** Four **distinct types**, so a value of one can never be passed where another is expected —
the mixing that SPEC-5 forbids is a compile error, not a runtime check. Each carries its own radius in
metres and refuses a non-finite or non-positive radius at construction. The one rule that *cannot* be a
type is the width relation above, and it is a named refusal.

## C4 — The work is counted where it happens, and the budget refuses

```text
WorkLedger { columns_produced, samples_read, cells, rectangles_merged, quads, vertices, indices,
             hits, misses, refusals }
Budget { max_columns: u64, max_quads: u64 }
Effort { budget: Budget, workers: usize }        // the two knobs of one decision: what a request may cost

derive(world, chunk, level, effort, resident, ledger) -> Result<Mesh, Refusal>
derive_many(world, chunks, level, effort, resident, ledger) -> Result<Vec<Mesh>, Refusal>
```

**Contract.** Every counter is incremented at the place the work happens — never computed from the
mesh afterwards — so the ledger answers *what was actually done*. `columns_produced` is what the field
plane produced for the patch; `samples_read` is what the merge actually read (a coarse level reads
fewer); both are reported when they differ, because one number would hide the difference (FR-008's
"what it actually did rather than what it intended").

**Budget.** A request that would exceed `max_columns` or `max_quads` is refused by name
(`geometry_budget_exceeded`) with both the needed and the allowed numbers, and **nothing is cached**:
a partial mesh is a wrong mesh, and a cached partial mesh is a wrong mesh that survives.

**Distribution.** `derive_many` derives several chunks across `workers` threads and returns them in the
**declared order** of the request; the worker count changes no quad and no digest (FR-003). A refusal in
any chunk is the call's refusal, reported by name rather than hidden in one slot of a vector.
