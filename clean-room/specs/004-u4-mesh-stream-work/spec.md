# Feature Specification: U4 — Meshing, streaming and work

**Feature Branch**: `004-u4-mesh-stream-work`

**Created**: 2026-10-11

**Status**: Draft — **written from the record, not from the legacy code.** The laws and boundaries in
this spec come from `docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5 and §1.3, the U4 entry of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 and slice 4 of its §5, and the reasoning recorded in
`docs/LEDGER.md` C-43 (a quad is the merge's unit; the GPU's unit is a triangle; a *displaced* surface
is not planar) and C-46 (a measurement that does not say which world it is of is not evidence). No
`.rs` under `crates/voxel-mesh/`, `crates/voxel-stream/` or `crates/voxel-work/` was read.

**Input**: User description: "U4 meshing, streaming and work: the geometry a world's fields imply, the
resident geometry kept coherent with the world it came from, and the work that produces it budgeted and
distributed."

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 *U4 — Meshing, streaming and work*, slice 4 of §5.

## What U4 is, and what it is not

U4 turns a world into **geometry** and keeps that geometry **coherent** with the world it came from,
under a **budget** that is reported honestly. Three boundaries are inherited whole and are the whole
reason this slice is worth having:

1. **One world authority (SPEC-3).** U2 owns truth (`World`: identity, tick, the anchor, the residency
   cache) and U3 owns the fields. U4 **consumes** both and grows no second copy of either. There is no
   terrain function in this slice — a mesh comes from the declared fields or it does not exist.
2. **Four horizons, computed not copied (SPEC-5).** The render horizon, the simulation horizon, the
   residency horizon and the field horizon are **four separate typed things**. They are not one radius
   with four names, and a value that belongs to one is never silently used as another.
3. **A measurement of the wrong world is worse than none (C-46).** Every number U4 prints says which
   world it is of: seed, declared period, tick, chunk and level.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The geometry a world's fields imply (Priority: P1)

A caller asks for the geometry of one chunk of a world. The answer is derived from the **declared
fields** of that world — the same `elevation`, `water_level` and `lithology` the shapers produce — and
nothing else: no second terrain function, no stored copy of the surface. The derivation merges equal
cells into rectangles (a quad is the *merge's* unit: `LEDGER.md` C-43 measured 34,816 visible faces
becoming 6 quads on a flat slab), and its material comes from U2's admitted registry. **(§39,
    2026-10-13:** the merge's unit is the **face** between a solid cell and air — a rectangle is a
    rectangle of a chunk's own voxel faces — and the column merge is kept **as the reading** (`U4
    reading:`) rather than as a second mesh.)

**Why this priority**: geometry is the first thing in this repository that is *about* a world without
being the world. If it can be derived twice and differ, or derived from a private copy of the terrain,
every later system (light, shell, instruments) stands on sand.

**Independent Test**: derive one chunk's geometry twice, at 1 and at 8 workers, and compare the
geometry digest; then move the anchor and U2's residency cache and derive again — the digest must not
move.

**Acceptance Scenarios**:

1. **Given** a world and a chunk, **When** its geometry is derived twice, **Then** the two meshes are
   identical quad for quad and their digest is one value.
2. **Given** the same world and chunk at 1, 2, 4 and 8 workers, **When** each derives, **Then** the
   digest is the same value — the worker count is a knob.
3. **Given** a chunk whose surface is flat and uniform, **When** it is meshed, **Then** the whole
   surface merges into one rectangle, and the printed merge factor says how many columns became how
   many quads.
4. **Given** a world whose fields a declaring stage refuses, **When** geometry is asked for, **Then**
   the stage's own refusal is passed through by name — never replaced by an empty mesh.

### User Story 2 - Resident geometry stays coherent with the world it came from (Priority: P2)

Deriving geometry is expensive, so it is kept — but a kept copy that can drift from its world is worse
than no cache at all (SPEC-3). Each resident entry records **the world state it was derived from**, and
it is served only while that state is current. A request in an unchanged world does **no** derivation
work; a world that has moved invalidates **exactly** the geometry derived from the old state and
nothing else; a stale entry is rebuilt, never served.

**Why this priority**: this is the difference between a streaming system and a memory leak that draws
yesterday's mountain. It is also where a second world copy would hide, so the property is stated as a
refusal (never serve stale) rather than as a hope.

**Independent Test**: derive three chunks, re-ask for one (a hit, zero columns), move the tick, ask
again (a miss and a rebuild), then invalidate and check that exactly the entries from the old state
went and the current ones stayed.

**Acceptance Scenarios**:

1. **Given** three resident chunks at one world state, **When** one chunk is asked for again, **Then**
   the entry is served as a hit and the ledger records **zero** columns produced.
2. **Given** resident geometry, **When** the world advances to a new tick, **Then** every request
   rebuilds and the ledger records zero hits, and when the world is invalidated **exactly** the stale
   entries are removed — the count is printed and equals the number of resident chunks from the old
   state.
3. **Given** resident geometry, **When** U2's anchor moves and its residency cache is populated,
   **Then** the geometry is still a hit and its digest is unchanged: a view is not truth.
4. **Given** a chunk outside the declared residency horizon that is **not** resident, **When** it is
   asked for, **Then** the request is refused by name, and no entry is created. Geometry that *is*
   already resident is still served: the horizon gates **derivation** (do not build what will not be
   kept), while the anchor is a view and not an input to an answer (FR-007).

### User Story 3 - The work is budgeted, distributed, and reported honestly (Priority: P3)

A caller declares a budget (a ceiling on the columns and the quads one request may produce) and a
worker count. The budget is a **refusal**, not a truncation: a request that would exceed it is refused
by name with both numbers in the detail. The work accounting counts what the derivation **actually
did** — field columns produced, samples read, rectangles merged, quads emitted, vertices and indices,
cache hits and misses, refusals — and those counts must agree with an independent recount of the
returned geometry, not with a formula.

**Why this priority**: the plan's slice 4 check is *"merge factor and per-chunk memory re-measured"*,
and C-46 is the record of what happens when a number is reported about the wrong thing. Work that is
only intended is not measurable.

**Independent Test**: read the ledger after a derivation, recount the mesh's own quads/vertices and the
chunk's own columns, and compare; then trip the budget and read the refusal's numbers.

**Acceptance Scenarios**:

1. **Given** a derivation, **When** the ledger is read, **Then** `quads == mesh.quads()`,
   `vertices == 4 × quads`, `indices == 6 × quads`, and `columns_produced` equals the field plane's own
   patch area — each checked by an independent recount.
2. **Given** a budget smaller than the request needs, **When** the geometry is asked for, **Then** the
   request is refused by name with the needed and allowed numbers, and nothing is cached.
3. **Given** a request that reads fewer samples than the field plane produced (a coarse level),
   **When** the ledger is read, **Then** it reports both numbers — what was produced and what was read
   — rather than one number that hides the difference.

### Edge Cases

- **A chunk of zero columns**: refused — an empty request is not a request (`site_alone`, inherited).
- **A level past the declared render ladder**: refused by name (`level_off_ladder`); never clamped to
  the coarsest level.
- **A lithology class the material registry has not admitted**: refused by name (`unadmitted_material`)
  with the class in the detail — a mesh never invents a material.
- **A budget exceeded**: refused by name (`geometry_budget_exceeded`) with both numbers; no partial
  mesh is returned and nothing is cached.
- **A chunk outside the residency horizon**: refused by name (`geometry_out_of_residency`) — when
  deriving it would be work. A resident entry is served, because nothing is being built.
- **A render horizon wider than the residency horizon**: refused by name (`horizon_mixed`) — rendering
  geometry the resident set will never keep is the mistake SPEC-5 exists to prevent. The four horizons
  are separate **types**, so identity mixing is a compile error rather than a runtime check.
- **A displaced (256×) surface**: the block level is planar and a rectangle is exactly two triangles, so
  this slice meshes it as quads; a *displaced* surface is not planar and must be subdivided and
  triangulated with normals from the displaced geometry (C-43). **Owed and named** in the plan.
- **Two chunks that share a border**: each is derived from its own columns, so a seam between them is a
  *declaration* (which chunk owns the marching edge), not an accident. This slice derives one chunk per
  request and states the seam rule in the contract; the stitched multi-chunk mesh is owed. **(§40,
  2026-10-13:** the stitch now **lands** as one footprint over a rectangle of chunks (`U4 region:` in the run),
  so the contract's seam rule is measured rather than only stated; a stitched mesh **wider** than one rectangle,
  with its own residency, is owed.**)**

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Geometry MUST be derived from the world's **declared fields** through the field plane's
  one seam; U4 MUST NOT sample terrain by any second path.
- **FR-002**: One geometry digest per `(world truth, chunk, level)`: the same inputs MUST produce the
  same quads and the same digest.
- **FR-003**: The worker count MUST NOT change the geometry or its digest.
- **FR-004**: A quad's material MUST come from U2's admitted material registry; an unadmitted class MUST
  be refused by name.
- **FR-005**: A resident entry MUST be served only while the world state it was derived from is current;
  a stale entry MUST be rebuilt, never served.
- **FR-006**: Invalidation MUST remove exactly the entries derived from a state that is no longer
  current, and MUST keep every entry derived from the current state.
- **FR-007**: U2's anchor and residency cache MUST NOT be geometry inputs (SPEC-3: a view is not truth).
- **FR-008**: Work MUST be counted where it happens; every count MUST equal an independent recount of
  the returned geometry.
- **FR-009**: A request that would exceed the declared budget MUST be refused by name with both numbers,
  and MUST cache nothing.
- **FR-010**: The four horizons MUST be four types; the derived check MUST refuse a render horizon
  wider than the residency horizon, and a chunk outside the residency horizon MUST be refused by name.
- **FR-011**: U4 MUST reuse the U3 stage/field vocabulary (`FieldId`, `Patch`, `ProcessRequest`,
  `Refusal`/`RefusalSet`) rather than inventing a parallel one, and MUST let U2 keep truth.
- **FR-012**: `cargo test --offline` MUST pass, `cargo clippy --offline --all-targets -- -D warnings`
  MUST be clean, and the binary MUST print U4's measurements and exit 0.

### Key Entities *(include if this feature involves data)*

- **`Chunk`** (exists, U2): a 32³ voxel region named by `ChunkKey`; the unit of residency and ownership.
- **`FieldId`** (exists, U3): the declared fields a mesh is derived from — `elevation`, `water_level`,
  `lithology` are the three this slice reads.
- **`Level`** (new): the render ladder — level `L` merges cells into rectangles of up to `2^L` columns
  (the coarsening the merge itself performs). A level past the declared ladder is refused by name.
- **`Quad`** (new): the merge's unit — a rectangle on one layer (`Solid`/`Water`) at one block height,
  carrying a material. Vertices are its four corners; triangles are two of it (C-43).
- **`Mesh`** (new): the quads of one chunk at one level, with the digest that identifies them.
- **`GeometryKey`** (new): `(world truth digest, chunk, level)` — **what a resident entry is bound to**.
- **`Resident`** (new): the kept geometry, keyed by `GeometryKey`, with the invalidation rule.
- **`WorkLedger`** (new): what the derivation actually did — produced, read, merged, emitted, hit,
  missed, refused.
- **`Budget`** (new): the ceiling one request may spend, refused by name when exceeded.
- **Four `Horizon` types** (new): render, simulation, residency, field — separate types with a named
  refusal for the one derived rule that can be checked at run time.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** One world, one chunk, one level ⇒ **one geometry digest**, and the same digest at 1, 2, 4
  and 8 workers. *Decided by:* the digest printed for each worker count (and `C-46`: each line names its
  seed, tick, chunk and level).
- **SC-002** A world that has moved invalidates **exactly** the geometry derived from it. *Decided by:*
  the printed hit/miss/rebuild counts — a same-state re-request is a hit with **0** columns produced; a
  new tick is a miss and a rebuild; the invalidated count equals the resident entries from the old
  state, and the current state's entries survive.
- **SC-003** The merge factor is measured, not claimed. *Decided by:* the printed `columns → quads`
  count for a flat uniform chunk (expected: many columns → one quad), with `vertices = 4 × quads` and
  `indices = 6 × quads` exactly.
- **SC-004** The work accounting is real. *Decided by:* an independent recount in the test — the ledger's
  quads/vertices/indices/columns must equal the mesh's own and the patch's own, and a hit must add zero
  work.
- **SC-005** Budget and horizon mistakes are refused by name, with their numbers. *Decided by:* the two
  refusals in the print, each with both numbers in its detail.
- **SC-006** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is
  clean, and `cargo run --offline` exits 0 with U4's measurements.

## Assumptions

- **The block level is planar.** A rectangle is exactly two triangles, so this slice's merge unit is a
  quad; the *displaced* 256× surface is where triangles become load-bearing (C-43) and is owed.
- **The fields are the truth.** U4 reads the declared fields at the resolution U3 produces them. A
  **strided** field request (a coarse level that does not compute columns nobody reads) would break every
  neighbour law in U3's roster — a patch is a dense grid and a slope needs its neighbours — so a coarse
  level reads the full patch and *decimates*, and the ledger reports both numbers honestly. The strided
  request is named in the plan as owed, and it belongs in U3's vocabulary if it is ever added.
- **Residency is U2's word.** `World`'s anchor and residency cache stay U2's; U4's resident set is
  geometry, keyed by the world state it was derived from.
- **One chunk per request**, derived from its own columns: the seam between two chunks is a declaration
  **(§40, 2026-10-13:** a chunk's own columns *and* the ring that decides its faces are what the contract
  declares, and a rectangle of chunks is stitched as one footprint; a wider stitch needs a region's own
  residency, and that is owed.)**
- **What the world looks like is the user's.** The meshing *mechanism*, the identities and the refusals
  are this feature's; the material palette and the terrain are the user's.
