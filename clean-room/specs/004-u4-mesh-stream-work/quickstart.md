# Phase 1 — Quickstart: validating U4

**Feature**: `004-u4-mesh-stream-work` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Read this first.** U4's first slice is **built and measured** (2026-10-11): the field-derived mesh
with its greedy merge, the level ladder, the four typed horizons, the resident set with exact
invalidation, the work ledger and the budget. Every scenario below carries **pasted output** from a run
of the commands above it. Four things are **owed and named** — the displaced 256× level, the
multi-chunk stitched mesh, a strided field request (U3's vocabulary) and the streamed residency policy
(U5/U6's) — and the binary prints them, because an owed capability is a fact about the build.
**(§40, 2026-10-13:** the stitch lands as **one footprint over a rectangle of chunks**; what remains owed is
a region's own residency rather than its one-footprint stitch — see Scenario 1).**

## Prerequisites

```bash
cd clean-room                # its own Cargo workspace, beside the legacy tree
cargo --version              # the toolchain the workspace pins
```

No network is required: the crate depends only on `sha2`, already resolved locally (`--offline`).

## Scenario 1 — the geometry a world's fields imply, one digest per world (SC-001, SC-003)

```bash
cargo run --offline
# measured 2026-10-11 (the binary takes no arguments; the world is the reference declaration's):
#   U4 render ladder: 3 levels (rectangles up to 4 columns); palette 6 classes over 5 admitted ground
#     materials: 0->1 1->1 2->2 3->3 4->4 5->6; water = 5
#   U4 horizons (four types): render 128 m < simulation 192 m < residency 256 m < field 1024 m,
#     ordered = true
#   U4 seed=1592639215 period=1s tick=0 chunk=(0, 0, 0) level=0: 144 quads from 1024 columns
#     (1031 cells, merge factor 7.16), vertices 576, indices 864,
#     digest 0e7358e45550878105b7ee0ee7f53b1039c1629dbdb059b2e7d333f31f814ed8
#   U4 worker knob: digest 0e7358e45550878105b7ee0ee7f53b1039c1629dbdb059b2e7d333f31f814ed8
#     at 1/2/4/8 workers is one value = true
#   U4 seed=1592639215 tick=0 chunk=(0, 0, 0) level=2: read 64 of 1024 produced columns, 27 quads,
#     digest 28c02ca285eb208b14f7f72073f481750f89ea40179d0a464710defc6b7332b2
#     (a level is part of the identity: true)
```

**Passes when**: one world and one chunk print **one** digest at 1, 2, 4 and 8 workers; the merge
factor is a measured ratio rather than a claim; and a coarse level reports **both** numbers it is made
of. **Measured:** all hold. `1024 columns → 144 quads` is 7.16 columns per rectangle; the blocks are
`1031` cells because five columns are flooded, and a flooded column enters **two** masks (the ground it
stands on and the water above it). `level 2` reads 64 of the 1024 produced columns — the decimation is
declared, and the difference is visible rather than hidden.

**(§39, 2026-10-13 — the mesh's unit is the cells' own voxel face; the block above is the reading's own
output, kept dated).** The mesh now merges the **faces** between a solid cell and air, greedily, in one
mesher keyed by the cells' own state digest; the **column merge survives, named a reading**
(`U4 reading:`), and there is no second mesh path. The same request prints `1024 columns read (1024
samples) → 11054 cells swept → 3703 faces → 3297 merged away → 406 quads (259 merged, factor 9.12),
vertices 1624, triangles 812, digest 2db62b78…b79c`, keyed by the reading's own state `146ff7bf…381733`;
over the cells' own door it prints `11054 cells swept → 3047 faces (2930 solid + 117 water) → 2648 merged
away → 399 quads (250 merged, factor 7.64), vertices 1596, triangles 798, digest 1172ac59…864c`, keyed by
the cells' state `9d2eea8c…f265`; `level 2` reads 64 of 1024 columns, sweeps 11168 cells → 3680 faces →
106 quads, digest `12e94e27…e786f`; and the recount line re-derives every figure from the cells
(`U4 face recount: … = true`).

**(§40, 2026-10-13 — one declared boundary-and-span contract, and the stitch across two chunks).** What is
meshed is now a `Footprint`: a patch of columns **and the band those columns imply**, both snapped to the
chunk boundary, with the ring of cells around it (`MESH_RING`) deciding its faces. The reading's door reads
that ring, so the same request prints `footprint 32×32 + ring 1 (1156 column(s) in the halo, 1156 produced,
1024 samples read), span 64..96 (1 band(s), 32768 cells), halo owned = true; 11054 cells swept → 1289 faces
→ 976 merged away → 313 quads (167 merged, factor 4.12), vertices 1252, triangles 626, digest
44a682e1…4ffb`; the cells' own door prints `world patch 48×48 (2304 column(s), 1024 under this chunk);
footprint 32×32 span 64..96 …, halo owned = false → 11054 cells swept → 2023 faces (1906 solid + 117 water)
→ 1696 merged away → 327 quads (181 merged, factor 6.19), digest 5ef04729…dbe0`. Over **one world** — the
chunk and its ring, built from the same fields by the same constructor — the two doors draw the same list,
byte for byte: `U4 one world, two doors: … = true; the reading swept 11054 cell(s) in 1289 faces, the cells'
door 11054 cell(s) in 1289 faces, and their rectangles are the same list, byte for byte (313 quads)`.

**The audit's measured disagreement is settled by the line beside it.** The audit measured 3 703 faces
through the reading's door and 3 047 through the cells' for the same 11 054 swept cells; the new extent line
prints one footprint in three worlds — `a world that is only the chunk (halo owned = false): 2679 faces; the
chunk + 1 ring (halo owned = true): 1289 faces, the same list as the reading's = true; the law's own world
(halo owned = false): 2023 faces`. The old 3 703 was the reading's world being the chunk *alone*, its own
edge drawn as a cliff — which is exactly what reading the ring hides; under the one rule the two doors agree
whenever they read the same world.

A rectangle of two chunks stitches in one footprint: `U4 region … 2 chunk(s) level=0: footprint 64×64 span
64..96 (1 band(s), 131072 cell(s)) → 44187 cells swept → 8013 faces → 6786 merged away → 1227 quads,
vertices 4908, digest 969a29a6…007f; the shared boundaries at x=32 and y=32 carry 6 rectangle(s), the cliffs
the two sides really show; the 4 chunk(s) carry the same 6 on those planes = true; the 4 chunk(s) meshed one
by one over the same world draw 8013 faces, the region 8013 = true`. `level 2` reads `64 of 1156` produced
columns, `11168 cells swept → 1280 faces → 68 quads`, digest `060784a0…7a0f`; the tick-1 rebuild is `2830
faces → 339 quads`, digest `36261871…a1a5`; the budget refusal now names `1156` columns; and the recount line
re-derives every figure (`U4 face recount: … = true`).

## Scenario 2 — the ledger equals an independent recount (SC-004)

```bash
cargo run --offline | grep "U4 recount"
# measured 2026-10-11:
#   U4 recount: ledger == mesh == the request's own columns = true
```

**Passes when**: `quads`, `vertices`, `indices` and `cells` in the ledger equal a recount of the
**returned mesh**, `columns_produced` equals the field plane's own patch area, `vertices = 4 × quads`
and `indices = 6 × quads` hold on the mesh itself, and a hit adds **zero** work. **Measured:** all
hold, in `src/mesh.rs`'s `the_ledger_equals_an_independent_recount` and in
`tests/u4_geometry.rs`, which recounts the reference world's own mesh.

## Scenario 3 — resident geometry stays coherent with its world (SC-002)

```bash
cargo run --offline | grep "U4 seed=1592639215 cache"
# measured 2026-10-11:
#   U4 seed=1592639215 cache: re-ask was a hit producing 0 more columns and 0 more quads (free: true);
#     at tick 1 the tick-0 entry is served = false (never stale); rebuild digest
#     4de4e860bed0db19e2e028497bc788f0deece433314500b78a7e0a46e06a8811 moved = true;
#     invalidate removed 1 of 2 and kept 1 current
```

**Passes when**: a re-ask in an unchanged world is a **hit with zero columns produced**; a moved world
serves **nothing** stale and rebuilds; and invalidation removes **exactly** the entries derived from the
world that is gone, keeping the current ones. **Measured:** all hold — `0 more columns and 0 more
quads`, `served = false`, the rebuild's digest moved, `removed 1 of 2` and `kept 1`. A moved **anchor**
changes nothing: `the_anchor_is_not_a_geometry_input` (unit) and the reference end-to-end check both
hold the digest fixed while U2's anchor moves and its residency cache fills.

## Scenario 4 — budget and horizon mistakes are refused by name, with their numbers (SC-005)

```bash
cargo run --offline | grep "refusal by name\|horizons mixed"
# measured 2026-10-11:
#   U4 budget refusal by name: [geometry_budget_exceeded] this request would produce 1024 columns and
#     the declared budget allows 64 (nothing cached: true)
#   U4 residency refusal by name: [geometry_out_of_residency] chunk (2, 0, 0) is 64.0 m from the anchor
#     at (0, 0, 0), outside the declared residency horizon 48.0 m (nothing cached: true)
#   U4 horizons mixed refused by name: [horizon_mixed] the render horizon 300.0 m is wider than the
#     residency horizon 256.0 m: geometry would be drawn from a set that is never kept
```

**Passes when**: each refusal names its kind, carries both numbers, and caches nothing. **Measured:**
all three. The column ceiling is checked **before** the fields are read (so `columns_produced` stays 0
for that refusal); the quad ceiling is checked **after** the merge, and that refusal carries the merge
that really happened (`the_budget_that_cannot_be_met_is_refused_with_its_numbers` asserts the work is
counted). A budget is a ceiling on **work**: a request already resident is a hit and has no work to
budget, which the reference end-to-end check states by measuring a fitting budget as met.

## Scenario 5 — the checks (SC-006)

```bash
cargo test --offline
# measured 2026-10-11:
#   lib:      132 passed; 0 failed (19 of them U4's)
#   u4_geometry: 2 passed; 0 failed (the reference world, end to end)
cargo clippy --offline --all-targets -- -D warnings
# measured 2026-10-11: clean (no diagnostics)
cargo run --offline
# measured 2026-10-11: exit 0, "Clean room OK: every built system's acceptance bar is measured and
#   passing."
```

**Passes when**: the tests pass, clippy is clean under `-D warnings`, and the binary exits 0. **Measured:**
all three, on 2026-10-11.

## What U4 still owes, measured rather than implied

```bash
cargo run --offline | grep "U4 owed"
# measured 2026-10-11:
#   U4 owed after this slice: the displaced 256x surface (a non-planar rectangle must be subdivided and
#     triangulated), the multi-chunk stitched mesh, a strided field request (U3's vocabulary), a
#     streamed residency policy (U5/U6's) and the per-chunk memory figure (slice 4 asks for the merge
#     factor and the bytes; this slice measures the quads, not their footprint)
```

- **The per-chunk memory figure**: the governing plan's slice-4 check asks for *"merge factor and
  per-chunk memory re-measured"*. The merge factor is measured (`4.12` over the reading and `6.19` over the cells' own faces since
  §40; `9.12`/`7.64` were the same doors before the extent contract, `7.16` the height merge's); the **bytes** are not, so the
  claim is `unchecked` here and the binary says so. A footprint measured by a formula about a struct
  would be the same defect C-46 names, so it is owed rather than estimated.

- **The displaced 256× surface**: a quad is not planar there, so it must be subdivided and triangulated
  with normals from the displaced geometry (C-43). This slice meshes the block level, where a rectangle
  *is* exactly two triangles.
- **A region's own residency** (§40, 2026-10-13): the multi-chunk stitched mesh **lands** as one footprint
  over a rectangle of chunks — the shared boundaries meshed once (see Scenario 1) — but it is kept nowhere,
  because the resident set's unit is a chunk and a level. A stitched mesh *wider* than one rectangle, with a
  residency entry of its own and a streamed policy over it, is owed. The seam's marching rule is settled: one
  footprint's own columns declare ownership, rather than a shared edge being discovered.
- **The strided field request**: a coarse level that does not compute columns nobody reads. It is a
  change to what a patch *is* (U3's vocabulary), and a patch is dense because every landscape law needs
  its neighbours.
- **The streamed residency policy**: which chunks to keep, and when to evict. U4 owns *exact*
  invalidation; the policy is U5/U6's, and persistence is owed to whichever system owns durability.
