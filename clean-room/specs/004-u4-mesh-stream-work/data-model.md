# Phase 1 — Data model: U4 meshing, streaming and work

**Feature**: `004-u4-mesh-stream-work` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## What U4 owns, and what it only consumes

| Concern | Owner | How U4 uses it |
|---|---|---|
| world truth — identity, tick, declared period, anchor, residency cache | U2 (`World`) | read through `World::identity`, `tick`, `truth_digest`; never copied, never cached |
| the declared fields — `elevation`, `water_level`, `lithology` | U3 (`field_at`) | read through the one seam, over a `ProcessRequest::Region` |
| materials — admitted ids and their refusals | U2 (`MaterialRegistry`) | a mesh's material is an admitted id; an unadmitted class is refused |
| geometry, its level ladder, its residency and its work | **U4** | this feature |

## The entities

| Entity | Fields | Rule |
|---|---|---|
| `Level` | `0..=MAX_LEVEL` | level `L` merges up to `2^L` columns per rectangle; past the ladder ⇒ `level_off_ladder` |
| `Layer` | `Solid`, `Water` | a column under water emits **two** quads: the ground it stands on and the surface it shows |
| `Quad` | `layer`, `height: i32`, `x`, `y`, `width`, `depth`, `material: MaterialId` | the **merge's** unit: the largest equal `(layer, height, material)` rectangle in a mask row-major scan (C-43); `x`/`y` are **world columns** and `height` is the block `z` (`z` is up) |
| `Mesh` | `key: GeometryKey`, `quads`, `digest` | `vertices = 4 × quads`, `triangles = 2 × quads`, `indices = 6 × quads` |
| `GeometryKey` | `truth: Digest`, `chunk: ChunkKey`, `level: Level` | the name a resident entry is bound to; `truth` is U2's truth digest |
| `Resident` | `entries: Map<GeometryKey, Mesh>` | serves only a current-truth entry; `invalidate` removes exactly the entries whose truth is no longer current |
| `WorkLedger` | produced / read / cells / merged / quads / vertices / indices / hits / misses / refusals | counted **where the work happens**, never derived from the finished mesh |
| `Budget` | `max_columns`, `max_quads` | a ceiling, not a target: exceeding it is `geometry_budget_exceeded` with both numbers |
| `Effort` | `budget`, `workers` | the two cost knobs of one request, grouped so neither looks like part of the world: a hit has no work to budget, and the worker count moves no quad |
| Four horizons | `RenderHorizon`, `SimulationHorizon`, `ResidencyHorizon`, `FieldHorizon` | four types; the width relation between render and residency is a named refusal |

## The mask, and how the merge is defined

A chunk's columns are read from the fields once. Each column becomes a cell:

```text
height  = round(elevation)                       // metres → blocks, the one rounding rule
water   = round(water_level) > height            // the column is flooded
material= registry[class_of(round(lithology))]   // an unadmitted class is refused by name
```

Two cells merge only if `layer`, `height` and `material` all agree, so the merge never crosses a
material boundary and never hides a step in the ground. Rectangles are grown **right, then down** —
one scan, one rule, no second pass — and the merged rectangles tile the mask exactly (every cell
belongs to exactly one quad; the sum of the rectangles' areas equals the cell count, which is what the
work ledger's independent recount checks).

## State transitions

```text
(world, chunk, level, effort)
        │
        ├── resident entry with a current truth?  ──yes──▶ HIT  (0 columns produced, 0 quads) —
        │        served whatever the anchor says: the anchor is a view, not truth (FR-007)
        │
        └── no ─▶ residency horizon: is the chunk inside it? ──no──▶ Err(geometry_out_of_residency)
                  └── yes ─▶ fields via fields_at ─▶ cells ─▶ merge ─▶ quads ─▶ Mesh + digest
                        │                                  │
                        └── refusal? ─▶ Err(by name)        └── over budget? ─▶ Err(by name), cache nothing
                                                                    else: insert, MISS

world advances to a new tick / another identity
        │
        └── every entry whose truth ≠ current is stale: rebuilt on request, removed by invalidate()
            an entry whose truth == current is untouched (and the anchor is not consulted at all)
```
