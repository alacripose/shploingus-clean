# Phase 0 — Research: U4 meshing, streaming and work

**Feature**: `004-u4-mesh-stream-work` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**What this phase had to resolve.** The spec names three properties; this file records the decisions
that had to be *made* before a design could be written, each with its rationale and the alternative
that was rejected. All of them are re-derived from documents this repository already holds
(`docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5, `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U4, `docs/LEDGER.md`
C-43 and C-46) and from the field plane U3 landed — **no legacy source was read** (P7), and no
external source was needed.

## D1 — A quad is the merge's unit, and the block level is where that is exact

- **Decision**: the merge unit is a **rectangle** on one layer at one block height with one material.
  Vertices are `4 × quads`, triangles `2 × quads`, indices `6 × quads`.
- **Rationale**: C-43 measured 34,816 visible faces becoming **6** quads on a flat slab, and recorded
  the boundary in the same breath: a quad is the *merge's* unit while the GPU's unit is a triangle,
  and a **displaced** (256×) surface is not planar, so four corners cannot represent it.
- **Alternatives considered**: (a) emitting raw per-column faces — rejected, it throws away the
  measurement the record is built on; (b) emitting triangles directly — rejected as premature: at the
  block level a rectangle *is* exactly two triangles, so nothing is lost, and the claim
  `indices = 6 × quads` becomes a single checkable sentence. The displaced level is **named owed**.

## D2 — One evaluation for the three fields, not three reads of three runs

- **Decision**: extend U3's field plane with **one** widened seam — `fields_at(roster, identity, tick,
  &[elevation, water_level, lithology], request, workers)` — and derive every mesh through it. The
  narrow `field_at` stays and is implemented on top of the same checks.
- **Rationale**: a mesh is a statement about **one** run of the world. Three separate `field_at` calls
  would evaluate the roster three times and, worse, would make "this patch" mean three different runs
  — the exact class of defect C-46 is about. The seam belongs in the field plane (the producer of
  fields), not in the mesher (a consumer), so it is added there rather than reached across.
- **Alternatives considered**: (a) sampling the fields in the mesher — rejected: that is a second
  terrain path (SPEC-3); (b) three `field_at` calls kept consistent by a promise — rejected: a
  promise is not a seam.

## D3 — A coarse level decimates, it does not stride the request

- **Decision**: level `L` reads one column per `2^L` out of the **full** patch the field plane
  produced, and the ledger reports `columns_produced` and `samples_read` separately.
- **Rationale**: U3's patch is a dense grid because every landscape law reads its neighbours
  (`slope`, D8 receivers, priority flood). A strided request would hand those laws a lattice they
  cannot evaluate, so the honest coarse level reads the whole patch and merges less of it. The
  difference is exactly what the ledger exists to show.
- **Alternatives considered**: (a) a strided `ProcessRequest` — **not rejected forever**, but it
  belongs in U3's patch vocabulary (it is a change to what a patch *is*), and the plan names it owed;
  (b) hiding the difference by reporting one "columns" number — rejected outright (FR-008's point).

## D4 — The key names the world, so stale geometry is unreachable

- **Decision**: a resident entry is keyed by `GeometryKey { truth, chunk, level }`, where `truth` is
  U2's `World::truth_digest()`. `Resident::serve` takes the **world**, so there is no lookup that could
  return geometry from a superseded truth. `invalidate` removes exactly the entries whose truth is no
  longer current and returns the count.
- **Rationale**: "never serve stale" as a *rule a caller must remember* is a promise; as a *key* it is
  a property. Invalidation then becomes what it should be — memory honesty, with a count that can be
  printed — rather than the thing that makes correctness true.
- **Alternatives considered**: (a) a `stale: bool` flag checked before serving — rejected: a flag can
  be forgotten, a key cannot; (b) invalidating on every truth change by mutating the world (U2) —
  rejected: U2 owns truth and geometry is U4's cache (SPEC-3).

## D5 — The resident set is consulted before the horizon

- **Decision**: `derive` serves a hit **first**, then applies the residency horizon to work that would
  actually be derived. The horizon is a policy on **derivation**, not a filter on answers.
- **Rationale**: FR-007 says the anchor is a view and not a geometry input. If the horizon were checked
  first, moving the anchor would change *whether* resident geometry is returned — quietly making the
  view an input. It also reads correctly on its own terms: do not *build* what will not be kept.
- **Alternatives considered**: the horizon first (the version this implementation started with) —
  rejected after the end-to-end check contradicted US2 AC3: a moved anchor turned a hit into
  `geometry_out_of_residency`. The failing test found a real design fault, and the fault is recorded
  here rather than only in the diff.

## D6 — The four horizons are four types, and only one rule can be a refusal

- **Decision**: `RenderHorizon`, `SimulationHorizon`, `ResidencyHorizon`, `FieldHorizon` are four
  distinct newtypes (each refusing a non-finite or non-positive radius at construction). The one
  relation that cannot be a type — the render horizon must not be wider than the residency horizon —
  is `check_horizons` and refuses `horizon_mixed` with both radii.
- **Rationale**: SPEC-5 exists because one radius with four names is how a renderer ends up drawing
  geometry from a set that is never kept. Four types make identity mixing a **compile error**; the
  width relation is the residue, and it is named.
- **Alternatives considered**: (a) one `Horizon` struct with four fields — rejected: nothing then stops
  a caller passing the simulation radius where the residency radius belongs; (b) checking all four
  pairwise — rejected: the other relations are not declared properties of the world, and inventing
  them would be a rule nobody asked for.

## D7 — A palette thinner than the world's class count is a declared mapping

- **Decision**: `Ladder::declare` names one admitted material per lithology class; `Ladder::over_palette`
  lays a shorter palette over `class_count` classes in declared order (monotone, total, and visible in
  `Ladder::mapping`, which the binary prints). A class the ladder cannot name — including a negative
  one — is refused `unadmitted_material` with the class in the detail.
- **Rationale**: the reference world declares **six** lithology classes and the starter registry admits
  **five** ground materials. Refusing to mesh the reference world would be a refusal about the palette
  rather than about the world; inventing a material would be the plausible substitute P2 forbids. A
  declared, monotone, printed mapping is the honest third option: two neighbouring classes may render
  alike, and that is a *decision on the record*.
- **Alternatives considered**: (a) adding a sixth ground material to U2's starter set — rejected as a
  change to another system's declaration in service of this one's convenience; (b) refusing class 5 —
  rejected: it refuses a real world for a rendering reason and says so with the wrong name.

## D8 — The two cost knobs travel together, and the ledger counts where the work happens

- **Decision**: `Effort { budget, workers }` groups the ceiling and the worker count; neither is a
  field of the mesher. Every ledger counter is incremented at the site of the work (a rectangle's cells
  when the rectangle closes, a column when it is read), and the counts are checked against an
  independent recount of the returned quads. The column ceiling is checked before the fields are read;
  the quad ceiling after the merge; and a refusal **keeps** the work it measured.
- **Rationale**: putting `workers` in the mesher would make a performance knob look like part of the
  world, and putting `budget` there would make a policy look like a fact. Deriving counters from the
  finished mesh would only prove the formula, which is the failure C-46 records.
- **Alternatives considered**: (a) a `Budget` parameter plus a `workers` parameter, exactly as the
  first draft had it — kept until the compiler's own argument-count limit (clippy's `too_many_arguments`)
  pushed the two knobs into one type; the result is a *better* seam, and the alternative (an `#[allow]`)
  was never acceptable; (b) reporting only the produced columns — rejected: it makes a coarse level look
  like it read everything it produced.

## Open questions this phase hands to the implement pass

- **Persistence**: the resident set is in memory for the life of the process. A persisted cache would
  have to carry the same key, and the key is already the world's truth — so the shape is settled and the
  storage is owed to whatever system owns durability.
- **The streamed residency policy** (which chunks to keep, when to evict) is U5/U6's: U4 owns *exact*
  invalidation, not the policy that decides what to ask for.
