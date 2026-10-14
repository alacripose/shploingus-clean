---
patch_id: PATCH-040
title: The column world against the cubic-chunk format — the report the mission planner asked for, and the audit it must run before the next slice
status: applied
operation: report
target: shploingus/clean-room
base_ref: shploingus-clean@main
base_digest: sha256:8e1dd0f9a59b316c41577bb68441465293709f9f3e65e762d75a0d11ef37ec6b
dependencies: [PATCH-039]
source: user-directed-conversation
created_at: 2026-10-13
approval: user-directed
live_mutation: false
---

# Change

**The user's direction, verbatim, and it is the whole of this patch's authority:**

> *"end your turn to the mission planner immediatley. an audit is required because we are not making a 2d game,
> and the cubic chunks format is incompatible with the cubic chunks format, which will require a clean room project
> audit. Report this to the mission planner"*

This patch **is** that report, and its head says exactly what is applied and what is not: `operation: report`,
`status: applied` — **the applied change is the report itself**, which changed no source and no test, and which
is why the head declares a user direction rather than a proposal (`von.rs`'s own claim for that word is *"a user
direction is applied"*, and a record that carries the word at any other stage is refused by name).

What it *asks for* is an **audit**, which is **not** applied and is not claimed to be: the rulings below are the
planner's to run, as its own patch, with its own head. This patch states the finding in the record's own terms,
with the lines that carry it, so that audit starts from evidence rather than from a suspicion.

## The user's second direction, which rules the first of those rulings

> *"just to be clear, the cubic chunks format caused massive rendering bugs which is why the clean room branch exists-
> it is the reason why there was materials flickering in wgsl. This is why we can't use it"*

That is the reason the record was missing, and it settles what this report could only present as an open
question. The column world is **not** one of two candidate shapes: it is the shape whose failures are the reason
this branch exists. **The material flicker in the WGSL path is a *rendering* defect in the *appearance* path** —
and the appearance path is U1's — so the visible client this pass just built is a **correct client of the shape
the product refuses**, and the audit's ruling on the shape is made by the user rather than left to the record.

Two consequences follow, and both belong in the audit rather than in this report:

* **The mechanism is now a first-class finding.** *Where* a per-column material read reaches a shader — the
  column's single class, its single `lithology`-mapped material and its one normal — and makes a face flicker or
  a material change under a camera is the audit's to establish, so that the clean room's cubic build **cannot
  reproduce it**. This record's own `crates/voxel-client/src/shading.wgsl` (mtime 2026-10-11 18:26) is the file
  the flicker was seen through, and P7 keeps the witness a *witness*: the audit measures behaviour and documents,
  it does not read that file as source.
* **The column world becomes a *reading*, or it becomes nothing.** Everything built on it keeps its dated
  measurements (§36's own included), but it can no longer be extended: a per-column material or water read is a
  read **of a cubic world's surface**, and it must say so on its own face or be retired.

The finding in one sentence: **the clean room's built world is a column world** — a square patch of cells, one
lithology class and one water depth per column, meshed as a heightfield, saved as columns, drawn top-down —
**while the record's own coordinate law, the documents' settled chunk contract and the witness are cubic**:
`32³` voxels on three axes with an addressed `16³` detail lattice per voxel face. Both shapes are in this tree
today. Nothing in the record has ruled which one the product is, and **the user has now ruled that it is not the
2D one**.

# The finding, with the lines that carry it

## The cubic side — what the record already declares

| where | what it declares |
|---|---|
| `docs/DESIGN-SPEC.md:41` | **"Chunk = 32³ voxels** (settled first-target contract: 1 m voxels, 32³ chunks …)" — the chunk is a volume on three axes |
| `docs/DESIGN-SPEC.md:18, 30, 33` | the **256** detail lattice is `16³` samples per voxel face, *addressed, never stored dense* |
| `src/coordinates.rs:46–70` (U2, **built**) | `Coordinate::chunk()` and `within_chunk()` (`0..32` on **each** axis), and `chunks_reached_by()` — the containing chunk plus one neighbour per boundary face, at most three. Three axes, cubic chunks: this is the clean room's own coordinate law, written from the documents and not from the witness |
| `src/coordinates.rs:5–…` (U2, **built**) | the signed metre lattice in three axes, with `within_chunk` bounded by the same 32 the chunk key is |
| `crates/voxel-model/src/coord.rs:46–48`, `subvoxel.rs:10` (**witness**) | `16×16×16` within a subvoxel = **256** per face, and the record's own note that an earlier reading of *256 per axis* was corrected on 2026-10-05 to `16×16×16` |
| `crates/voxel-client/src/renderer.rs`, `crates/voxel-shapers/src/process.rs` (**witness**) | a camera into voxels; fields over chunks |

## The column side — what the last seven passes built

| where | what it builds |
|---|---|
| `src/field.rs:65, 98–123` | every field is a `Vec<f64>` **per column** (`field_at`), with `FieldId::{Elevation, Lithology, WaterLevel}` as column reads; the multi-field form is `Vec<Vec<f64>>`, still one vector per column |
| `src/being.rs::WorldRead`, `cells_digest` | the world a being reads is a patch of columns with per-column `elevation` and `water_level`, and the world's **identity** is a digest over exactly those per-column arrays |
| `src/automaton.rs:15, 32–38` | *"one water **depth per column** of the patch the read already named"*, and the tick moves water **one hop between columns** (§34–§35) |
| `src/mesh.rs:28–31, 44` | *"the block level is planar: a rectangle is exactly two triangles"*, and *"level `L` merges up to `2^L` **columns** into one rectangle"* — a **heightfield** quad mesh. Its `GeometryKey { truth, chunk, level }` already carries the cubic `chunk` key, so the two shapes are already in one struct |
| `src/surface.rs` (§36, this pass) | the visible client: **576 × 576** pixels from **2 304 columns**, one flat colour per column, the terrain's slope entering only through the column's own normal. The picture is a **top-down plan view** — no projection, no camera, no depth, no voxel faces |
| U9's saves (`465 bytes`), U10's rows, U12's beings | all read that column world, and §35 spent a whole repair on making **two per-column water truths into one** |

## What is *provable in this tree* now, and what the audit must establish

**Provable now, and already measured:** the clean room's coordinate law is cubic-and-three-axis while its field,
being, water, mesh and client layers are column-and-planar; the two meet inside `GeometryKey`, which carries a
voxel `chunk` key and a column-derived rectangle; and this pass's own client is a 2D plan view by construction
(§36's "one concern, one owner" table says *no camera* in as many words).

**Not provable from this tree, and the audit's job:** whether the **witness's** on-disk chunk format can be
expressed by the column world *at all* — a column cannot carry a vertical stack of materials, an overhang, a
cave, or the addressed `16³` detail lattice, so the expectation is that it cannot, but **that is a claim the
audit must measure rather than inherit from this report**. This patch deliberately does **not** read
`crates/` for the answer: Constitution **P7** makes the witness a *behavioural* witness and never a source, so
the audit's own evidence must be documents (`DESIGN-SPEC.md`, `SHAPERS.md`, `MODS-MAP.md`), declared formats and
the record's own measurements — with the witness read only where the record already permits it (a contract read,
not a copy).

# The audit's four rulings

The audit is not a rewrite and not a plan pass: it returns **four rulings**, each with the evidence behind it,
and the planner decides the order.

1. **The shape — ruled, and the column representation is refused for cause.** The product is the **cubic
   chunk**: `32³` voxels on three axes, the addressed `16³` detail lattice, a camera into voxels
   (`docs/DESIGN-SPEC.md:41`, `src/coordinates.rs`). The **column patch** is refused because it *broke the
   renderer* — the user's words above — so what the audit owes here is not a choice but two things: the
   **mechanism** of the flicker (a new first-class finding, named above), and a ruling on **how much of the
   column world survives as a *reading of* a cubic world** rather than as a world of its own.
2. **The identity.** `being::cells_digest` hashes per-column `elevation` + `water_level`; a cubic world's
   identity must be a function of chunks, not of a column set, or `cells_digest` must be **renamed and demoted**
   to what it actually is: the digest of one *surface* read over a cubic world. U2's identity, U9's save layout
   and U10's rows all inherit this ruling.
3. **The seam.** Which layer changes first. The candidates, from the record's own shape: `field.rs`'s column
   reads become column-*of-a-chunk* reads (a `(x, y)` stack in `z`), the automaton's flat per-column water plane
   becomes the volumetric state the row's own law needs, `mesh.rs`'s heightfield merge becomes a greedy merge
   over **voxel faces** (which `LEDGER.md C-43` already describes as *"34 816 visible faces became 6 quads"* —
   face merging, not column merging), and the client's plan view becomes a **projected** camera. The audit names
   which of these is the first, and what stays shut behind it.
4. **The client.** §36's client is a *correct* client of a column world and it is **not** the client a cubic
   product needs. The ruling is whether `src/surface.rs` is (a) kept as a **plan-view instrument** — a tool that
   reads a cubic world's surface and draws it, named as such — or (b) superseded by a projected camera over voxel
   faces, and in that case what the GPU pass is now being checked against.

# What is frozen pending the ruling

* **No further column-shaped slices.** §5 row 1's remaining piece (the **GPU pass**) and row 13's **`P8` gate**
  are both still column-shaped steps, and a shader checked against `presentation::shade` would be a shader for a
  picture whose shape is now refused **for cause** — the very per-column material read whose WGSL path flickered.
  They follow the ruling, and the GPU pass is owed as a shader over **voxel faces** rather than as a shader for
  this picture.
* **The captures stand as they are.** `docs/baselines/u1-visible-client/` (this pass's pair, the windowed five
  lines, the P6 screenshot), `u12-settled/`, `u12-stepping/`, `u12-repair/` and the rest are **measurements of
  the column world at a dated moment**. They are not deleted and not rewritten: §36's own record says what they
  measure, and §36 stands as a *column-world slice* rather than as a mistake.
* **The marsh ruling** (§35's first owed item — whether a world where 71.5 % of columns stay wet is the world
  the row wants) is now **second to the shape ruling**, because the shape decides what a column's water even
  means.
* **Nothing under `crates/` is written.** No source, no test, no format. The audit reads; it does not import.

# What this patch does not claim

It does **not** claim to have reproduced or diagnosed the flicker itself: the user reports it as the reason this
branch exists, and the mechanism is the audit's to establish. What it claims is what is measurable in this tree
today: **two shapes are in one tree, the record declares the cubic one, and the user has now ruled the column one
out for cause** — not on taste, but because a per-column material read is what reached a shader and flickered.
The column world's own numbers are real and dated — 2 304 columns, 1 647 of them wet at rest, 39 luminance bands,
66 355 200 of 66 355 200 pixels read back — and they are the measurements of the shape the product refuses,
which is exactly why they must be read as a **reading of** a cubic world, or retired.

---

*Reported by the clean-room agent at the end of §36 (2026-10-13), on the user's direction, at the user's
instruction to end the turn — and updated the same session with the user's second direction, which supplies the
cause this report had only been able to name as an open question: **the cubic chunks format is what produced the
material flicker in the WGSL path, and that is why the clean room exists.** The plan's §36 "Owed, and named" and
"The next buildable step" carry the same finding in the record's own words.*
