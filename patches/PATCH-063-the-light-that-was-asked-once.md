---
patch_id: PATCH-063
title: The light that was asked once — a refused cell is asked again, and the frame stops keeping a blacklist it never emptied
status: applied
operation: fix
target: shploingus/light-ask
base_ref: shploingus-clean@main
base_digest: sha256:69b3490839fdfa3708657d8d298b321a4c1c01f304cfce13afecd2ae96189953
dependencies: []
source: user-reported-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — shadow sampling broken, surfaces unexpectedly bright, shadows not re-sampling with the sun and moon"
live_mutation: false
---

# Change

**Diagnosis first, and it is not what it looked like.** Shadow sampling in this tree is not a shadow
map: there is no depth texture and no shadow pass. A corner carries **six measured bands** — the
direct triple after a beam has been marched through the resident octrees, and the sky's diffuse
triple — and the fragment shader reads them at `light_present == 1`, falling back to the rig
uniform when the cell has no measurement (`shading.wgsl`, the `which light` branch).

The path, traced end to end:

| what | where |
|---|---|
| the sun/moon direction | `crates/voxel-client/src/sun.rs` (`Body`, `toward_beam`), and the field takes it per atmosphere cadence |
| where a shadow is computed | `skylight::WorldLight::bands_across`, one march per **voxel**, up to `field::REACH_M` = 160 m, on `Measurer`'s own thread |
| where the field is refreshed | `App::pump_light` (adopt, service, ask) and `CellField::of_light` per cadence |
| what decides a sample is stale | `CellField::at`: `dot(kept.toward, toward) < BEAM_STEP_COS`, where the step is 0.5° — the solver's own stencil resolution |

**So the design does re-sample a moving light, and its own test says so**
(`field::tests::the_fields_coverage_survives_a_beam_swing_and_a_stale_answer_is_counted`). What broke
is one line of bookkeeping on the frame's side.

`App` held `light_asked: HashSet<(i32, i32, i32)>`, inserted on every address handed to the worker and
removed **only** when a measurement landed. But the worker's result carries no refusal arm:

```rust
struct Batch { measured: Vec<Measured>, last: bool }
```

`run` pushes a `Measured` only when `bands_across` returns `Some`; a cell whose own chunk is absent
contributes nothing and the ask still ends (`last: true`). The frame therefore never learned that the
cell had been refused, never removed it from `light_asked`, and **never asked for it again**. The
comment on the set admitted the leak and capped it at 65 536 addresses — against the resident
window's ~82 000 surface cells, so a walk could poison most of the field. Those corners take
`light_present == 0` for the rest of the run and are lit by the rig: **unshadowed direct sun and
ambient wherever the chunk had not yet arrived when the beam was asked** — bright patches with hard
mesh edges, and shadows that never re-sample because the measurement is never retried.

That is the whole of the reported symptom. It is a **format gap at a real boundary**, but not a musli
one: `grep -rn musli crates/voxel-client/src/{measure,field,skylight}.rs` and `shading.wgsl` → none.
The structs that cross the worker boundary are plain Rust over an `mpsc` channel, and the field they
lack is the one that would say *this cell was refused*.

**The fix, narrow.** The set is gone. Each batch is deduped locally (`dedupe_batch`), which is the
only thing the set was for while an ask was in flight; a cell the worker refused stays in the field's
own queue and is asked again at the next build.

# Acceptance

- `dedupe_batch` removes a duplicated address inside one batch and holds no state between batches, so
  a refused cell is offered again — `a_batch_is_deduped_and_a_refused_cell_is_asked_again`.
- A refused cell that is offered again lands once its chunk is resident —
  `measure::tests::a_refused_cell_is_offered_again_and_lands_when_its_chunk_arrives`.
- `App` carries no cross-ask memory of what it asked for.
- Nothing about the solver, the stencil, the step, the march or the shader changes.

# Evidence

- `cargo test -p voxel-client --lib` — **533 passed, 0 failed, 2 ignored** (the new refusal test
  included).
- `cargo test -p shploingus --bin shploingus` — **2 passed, 0 failed**.
- `cargo check --workspace` — **finished**, warnings pre-existing and untouched.

**Named, not fixed — the remaining open defects this diagnosis found:**

1. **The sky is added twice on a measured corner.** The shader adds the rig's ambient before the
   branch, then adds the field's own sky inside it (`colour += base * (beams * … + sky * opaque)`),
   with a comment that states the two are not yet commensurable. That is a real brightness overshoot
   on every measured cell and it wants a measurement of the two terms, not a coefficient chosen here.
2. **The result format has no refusal arm.** A skipped cell is still invisible; the frame now retries
   everything it still wants, which costs a repeated march per refused address but names nothing. The
   honest fix is `Batch { measured, refused }`, and it is a change to the worker's contract.
3. **Geometry re-samples at 1°, not 0.5°.** `GEOMETRY_STEPS = 2.0` in the atmosphere cadence: the
   field re-marches at the stencil's step, the drawn vertices only when the beam has swung a degree,
   because a rebuild is 228 000 corner asks. A shadow can therefore be up to a degree of sun behind
   the field it was measured from, by design and stated.

# Rollback

Restore `crates/shploingus/src/main.rs` and `crates/voxel-client/src/measure.rs` from `HEAD`, and
delete this record. The set returns, and with it the leak it had.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0954
site: crates/shploingus/src/main.rs:2406
what: keep the light ask's dedupe per batch, so a cell the worker refused is asked again at the next build
because: the result carries no refusal — a cell whose chunk is absent ends the ask with nothing in `measured` — so a run-long in-flight set kept its address out of every later batch and left its corners on the unshadowed rig for the whole run
check: cargo test -p shploingus --bin shploingus a_batch_is_deduped -> failures 0
witness: crates/shploingus/src/main.rs
expects: 0
requires: cargo test -p voxel-client --lib measure::tests
requires: cargo check --workspace
```
