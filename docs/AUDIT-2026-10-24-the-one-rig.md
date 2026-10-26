# Audit — the one rig, PATCH-063 … PATCH-073

Written 2026-10-24. Scope: the lighting, shadow and cell-shell head delivered in PATCH-063 …
PATCH-073, and the tree those records land in. The plan's other missions (M10 packages, LOD
materials, the 1/32 field) are **not** audited here; they are named as out of scope at the end.

Every number below was either re-run in this pass or is marked **quoted** with the record it comes
from. The distinction is the point of the audit: the corpus is unusually good at saying where a
number came from, and this pass found the places where the tree has moved out from under it.

---

## 0. Scorecard

| dimension | grade | one line |
|---|---|---|
| Evidence discipline | **A−** | records name the test, the run and the rollback; most claims reproduce |
| Claim accuracy | **C+** | three stale numeric families; one headline claim describes a configuration the binary does not draw |
| Ownership | **C** | the beam's shadow has two owners and the shader spends the map, not the lattice |
| Residue | **D** | 429 MB / 788 files of test scratch in a tracked directory; 2 328 build warnings; one leaking test |
| Test health | **B** | 558 lib tests pass; several self-documented gaps; one witness no longer measures what it names |
| Delivery state | **F** | 307 modified, 676 untracked, 3 deleted; HEAD is `BACKUP pre-flicker-rewrite` |

The grades are not a judgment of the work's ambition. They are what a reviewer could land from this
tree today without re-deriving it.

---

## 1. What was run in this pass

| command | result |
|---|---|
| `cargo test -p voxel-client --lib` (04:48, before this audit) | **558 passed, 0 failed, 2 ignored** in 232 s; integration **2 + 1 passed** |
| `cargo test -p voxel-client --lib the_frame_is_exposed_once -- --nocapture` | **1 passed** in 158 s; prints the night share and the twilight step |
| `cargo build --release -p voxel-client --example shadow-lattice` | **Finished** in 1 m 51 s; the source is newer than the previous binary |
| `shadow-lattice` on the rebuilt example | **exit 0**; both shoulders reproduce (below) |
| `grep` / `wc` census of residue, ignored tests, module sizes | §3, §4 |

Not run: the five-minute shipped binary (`SHPLOINGUS_FRAMES=1`), so PATCH-072's live corner counts
(1 289 453 → 2 050 887) and frame statistics remain **quoted**, not re-measured. The shell's screen
losses (PATCH-071) are prose in this pass; no frame was compared.

---

## 2. Claim checks

### 2.1 Reproduced on the current tree

The example's own transcript, rebuilt at 05:05 and run against the current source:

```
shipped fine:   the shadow's own footprint fades 90 % to 10 % over 7.1 px = 0.103 m (panel 411.3 px)
shipped coarse: the shadow's own footprint fades 90 % to 10 % over 282.4 px = 4.120 m
shipped cost:   244413 corner(s) / 423948 index(es) at the lattice in 170.1 ms and 5113375 ask(s),
                against 228700 / 343050 at lattice 1 in 31.5 ms and 457400 ask(s)
per chunk:      0.28 ms and 8452 ask(s) at the lattice against 0.05 ms and 756
synthetic:      41.0 -> 40.0 px = 0.195 m fine, 135.0 px = 0.659 m coarse
```

Against PATCH-073's record (242 060 / 411 954 corners, 4.146 m coarse, 41 646 differing pixels):

| claim | record | this pass | verdict |
|---|---|---|---|
| shipped fine shoulder | 0.103 m | **0.103 m** | holds exactly |
| shipped coarse shoulder | 4.146 m | **4.120 m** | holds, −0.6 % |
| asks a chunk, fine / coarse | 8 452 / 756 | **8 452 / 756** | holds exactly |
| milliseconds a chunk | 0.28 / 0.05 | **0.28 / 0.05** | holds |
| synthetic fine / coarse | 0.200 / 0.663 m | **0.195 / 0.659 m** | holds within a pixel |
| differing pixels | 41 646 (28.2 %) | **41 661 (28.3 %)** | holds |
| fine corners / indices | 242 060 / 411 954 | **244 413 / 423 948** | **does not match** (+1.0 % / +2.9 %) |
| tick | 9 000 | **81 000** | **does not match** (see §2.3) |

The shoulder claim is the load-bearing one, and it survives a rebuild and a re-run. The cost totals
are a build older than the tree; the record should either be re-stamped or its totals dropped to the
per-chunk figures, which are stable.

### 2.2 The headline claim: the lattice is not the shipped frame's beam shadow

PATCH-070 says *"a face is now cut at the light's own lattice"* and PATCH-073 says *"the sub-metre
structure the old path could not produce is in the shipped frame."* The first is true of the vertex
buffer. The second is not established, and the shader says why:

- [`shading.wgsl:1363`](../crates/voxel-client/src/shading.wgsl#L1363) starts `beams` from the
  field's `light_direct` — **and then**
- [`shading.wgsl:1365`](../crates/voxel-client/src/shading.wgsl#L1365) replaces it with
  `rig.lights[0] · sun_visibility(...)` whenever the rig carries a light **and**
  `beam_shadow.direction.w > 0.5`.
- [`main.rs:673`](../crates/shploingus/src/main.rs#L673) is the shipped binary: `set_shadows(true)`,
  with the range set to the window's own horizon — `(5 + 1) × 32 m` = **192 m**.

`shadow-lattice`'s shipped half never calls `set_shadows`; `grep -n "set_shadows" examples/shadow-lattice.rs`
is empty, and the example's own pipeline starts with shadows off (`world.rs:997`). So its fine frame
is the **shadows-off** configuration — which is exactly the configuration in which the lattice's
direct band reaches the pixels. In the binary the beam is the cascaded depth map (5.5 cm texels
inside 28 m; the far cascade reaches the frame's 192 m range). The field's **direct** band reaches a
pixel only in a frame with no beam to cast — shadows off (the example), or no rig light at all (the
moonless night); with the map on and a rig light present, both shader branches spend the map. The
field's **diffuse** band is still drawn wherever a corner has a measurement.

Consequences, stated plainly:

1. The 0.103 m is a real measurement of the lattice, and a real measurement of the **example**, but
   it is not the edge the player's beam casts. PATCH-073's "in the shipped frame" should be
   corrected to "in the shipped window with the beam shadow off", or the example should turn shadows
   on and re-measure. The shader's own comment (`shading.wgsl:1356`) states the design: *"the beam is
   the rig's, and its shadow is this frame's"* — the field's direct band is deliberately kept only
   for frames the map cannot serve. The records and the shader cannot both be describing the frame.
2. PATCH-070's "the frame asks at subvoxel resolution" is true of the corners and false of the
   pixel's beam. The two statements are not the same claim, and the record uses them interchangeably.
3. The 2.05 M-corner arena PATCH-072 bounds is paid, in the picture, for a direct signal the
   fragment then discards wherever the map is active. The lattice still earns its cost through the
   **sky** term (`light_diffuse` per corner), but that term cannot cast a directional shadow edge;
   it is a dome occlusion. The cost is therefore defensible for the ambient, not for the beam.
4. This is the same defect class PATCH-070 named for the walk — "the same question had two answers" —
   one level up: the answer is now split between the field and the map, and the shader picks the map
   without saying so.

### 2.3 Numeric drift, record versus tree

The world's scale moved after these records were written: `TICKS_PER_DAY` is **216 000**
(`climate.rs:49`) and `GAME_TICK` is `TICKS_PER_DAY · 3/8` = **81 000** (`game.rs:126`). Several
records still quote the 24 000-tick world:

| record | quotes | today | what survives |
|---|---|---|---|
| PATCH-066 | "`GAME_TICK = 9000`", first night `21 337` | 81 000; `21 337` is a 24 000-tick literal | the **ratio** survives: 3/8 day, epoch half a synodic month later, opening phase 1.0, elongation −180° |
| PATCH-073 | "at tick 9000", Sun 10.6° | 81 000, Sun 10.7° | the altitude and the panel are the same by construction; the tick literal is stale |
| PATCH-068 | "`-12°` is astronomical twilight's own end" | unchanged | holds |
| PATCH-072 | "settled at 2 050 887 live corners" | quoted, not re-run | holds only if the five-minute run still settles there |

The plan (`docs/PLAN-2026-10-23-the-one-rig.md:107–108`) records the 24 000 → 216 000 migration and
the tests it repaired. The **records** do not carry that note, so a reader who trusts them is reading
a world that no longer exists — the same failure PATCH-066 itself annotated so carefully when
PATCH-068 superseded it.

The corner arena has three figures for one quantity, and the vertex constant has a stale doc line:

| where | says | what it is |
|---|---|---|
| `scene.rs:1418` | "peaks at 1.25× live … 2.6 M corners … 178 MB" | the **bound** at 2 050 887 live (2 563 609; 174.3 MB) |
| PATCH-072:40 | "peaks at 1.25 × live (2 402 372 corners, 163.4 MB)" | the **observed** peak (1.17× live), labelled as the bound |
| `world.rs:2211` | "2.07 M corners are 168 MB at the arena's peak" | 2.07 M × 68 B = **140.8 MB**; 2 402 372 × 68 = **163.4 MB**; neither is 168 |
| `world.rs:2222` | "The corners **128 MiB** of vertices holds" | the constant directly below is **256 MiB** |

`Vertex::STRIDE` is 68 B (`scene.rs:151`), so the arithmetic is checkable and three of the four
numbers are wrong or mislabelled. None of them changes a bound's behaviour; all of them change what a
reader believes the bound is.

### 2.4 A dead factor in the Sun's own light

PATCH-065 says the drawn body's radiance is scaled by `twilight_fraction`, *"the same share
`daylight_of` takes for the rig's beam."* The code at
[`skylight.rs:240`](../crates/voxel-client/src/skylight.rs#L240) multiplies `visibility` into the
intensity, but line 241 gates the whole expression on `SkyPhase::sun_is_up` (`altitude > 0.0`).
`twilight_fraction` is exactly `1.0` for every altitude `>= 0`, and the branch is never taken below
it. **The factor is provably dead**: it cannot change the beam at any hour. The drawn body (in
`sun.rs`) does fade, and that claim holds; the rig's share does not exist.

This matters beyond tidiness: it is the sentence that makes PATCH-065 sound like the sunset's beam
was repaired, when the beam still switches at `0.000°` and the test at `skylight.rs:2232` prints the
quantity that no longer steps (`8.4051 → 8.3988 W/m²`, 0.075 %) while asserting only
`light_step > 0.0`. The rig's own intensity — the thing that goes to zero — is not printed there.

### 2.5 Lineage: PATCH-069's implementation is not in the tree

PATCH-069's change is `charge_shape` / `charge_cell` in `voxel-stream`; its rollback says *"delete the
`Node::Mixed` arm in `march_tree` and `charge_shape`/`charge_cell`."* Neither function exists.
PATCH-070 says so explicitly — *"PATCH-069's `charge_shape` is not in this tree: the split lives in
the walk that knows the geometry"* — and the split is now in `Spans::split`
(`voxel-mesh/src/raycast.rs:479`). The acceptance test survives and passes
(`light.rs:791`), so the **behaviour** is landed; the **record's mechanism and rollback** are not.
PATCH-069 should be marked superseded by PATCH-070, not left as an applied record whose rollback
cannot be performed.

The same class, smaller: PATCH-072's rollback says *"Restore `ARENA_SLACK = 2`"*; `ARENA_SLACK` no
longer exists (the bound is `ARENA_DEAD_IN`, `scene.rs:1420`). A rollback that names a deleted
constant is a sentence, not a procedure.

### 2.6 The rest of the checked arithmetic holds

- **Retint table (PATCH-071).** `#1B1922 → #1A1A1A` is 66; `#332F45 → #333333` is 340; the elevation
  ladder `#252525 → #1A1A1A` (363 vs 588) and `#484848 → #333333` (1 323 vs 9 408) all follow
  `retint`'s smallest squared-byte distance. The table is correct.
- **Capacity (PATCH-072).** 256 MiB + 64 MiB a pair × 2 = **640 MiB retained**; 256 MiB / 68 B =
  **3 947 808 corners**; 64 MiB / 4 B = **16 777 216 indices**. Correct.
- **Arena bound.** `dead > live / 4 + 4096` and the test's `held <= live + live/4 + 4096`
  (`scene.rs:1420`, `:3685`) agree.
- **Night share (PATCH-068).** Re-run: day ground **3.684**, brightest night **0.0549** at Sun
  −43.1° = **1.49 %** (record: 1.45 %); bound 5 %. The difference is drift, not a failed claim.
- **Grade.** The sweep's worst mapping step is **0.00000**; every hour is exposed at 1.0. Holds.
- **Cell shell.** `HOLO` is `[Rgb; 12]` and the module's own header says "twelve"; the item doc at
  `cells.rs:356` still says "**eleven** values". PATCH-071 named this and left it; half of the owed
  edit was made, the other half was not, and no record says so.

### 2.7 Test counts

PATCH-069/072/073 quote **543 passed**. The library test binary today reports **558 passed, 0
failed, 2 ignored** (232 s), plus the two integration tests (2 + 1). The suite is healthy; the
number in the records is a snapshot of an earlier tree.

---

## 3. Residue

### 3.1 Test scratch in a tracked directory

```
logs/test-workspaces/           390 directories, 788 files, 429 MB
  notifications-tampered-<pid>  373 dirs (one per run), ~2 files each
  shploingus-console-17360-file-prose  402 MB (one leftover console run)
```

`logs/` is tracked (the report files are), and `git check-ignore` does not match the scratch, so
`git status --untracked-files=all` carries all of it. The cause of the 373 directories is a test, not
a run: `a_tampered_artifact_is_refused_before_it_is_shown`
(`shell/notifications.rs:396`) creates `logs/test-workspaces/notifications-tampered-<pid>` and never
removes it: its one `remove_dir_all` is at the start, where it clears whatever a previous run left,
and there is none at the end. The stream test above it clears at the end and leaves nothing.

Smallest fix: one `remove_dir_all` at the end of that test, one `logs/test-workspaces/` line in
`.gitignore`, and delete the existing tree. The 402 MB console directory is a separate leak from a
`file-prose` run and should be found before the next one.

### 3.2 Ignored tests

**20** `#[ignore]` attributes. Thirteen are deliberate canonical-record writers (`file_*`,
`migration`, `chain_repair`, the mount's ledger filing); seven are measurement sweeps (`skylight`,
`terrain`, `resident` ×2, `world_source`, `work` ×2). The sweeps are the ones to watch:
`skylight::measure_the_day_and_the_night` is the only witness for some of the figures quoted above,
and nothing runs it.

### 3.3 Build noise

A release build of one example emitted **2 328 warnings**, of which 2 307 are `missing_docs` in
dependency crates (905 struct fields, 540 variants, 531 methods). Buried in them: one unused import
(`sky_inputs_at`), four unnecessary-parenthesis warnings, and no way for a reviewer to see a new
warning arrive. The workspace's `#![warn(missing_docs)]` is a real policy; applying it to every crate
at once makes the policy unreadable.

### 3.4 Dead weight

| item | size | state |
|---|---|---|
| `voxel-client/src/fog_cubes.rs` | 265 lines, 2 tests | **no call site**: the only reference is `pub mod fog_cubes` in `lib.rs:68`; its header says fog "was not cubic in the picture" and it still is not |
| `skylight.rs:240` `visibility` | one factor | dead below the gate (§2.4) |
| `world.rs:2222` "128 MiB" | one doc line | contradicts the 256 MiB constant below it |
| `cells.rs:356` "eleven values" | one doc line | contradicts `[Rgb; 12]` and the header |
| two `#[allow(dead_code)]` in `world/change.rs` | — | explicitly parked for "the next landing"; named, so not hidden |

The fog module is the one worth a decision. It is not residue in the sense of "left behind"; it is a
finished feature that no frame calls, with a doc that claims a picture it is not in.

---

## 4. Structure

### 4.1 Size

```
voxel-client src: 50 966 lines
  shell/mount.rs      7 403
  renderer.rs         4 204
  scene.rs            4 086
  skylight.rs         3 919
  world.rs            2 938
  cloud_cubes.rs      1 738
  shell/window.rs     1 704
  motion.rs           1 676
  shell/arrangement.rs 1 594
shploingus/src/main.rs 3 951
```

`mount.rs` is 7 403 lines: the largest file in the client and the one that composes the shell, owns
the package loader, the windows, the HUD and the notifications' consumer. It is not one concern with
a big test module; it is several concerns sharing a file. The same is true, less severely, of
`scene.rs` (geometry, lattice emission, residency, arena) and `renderer.rs` (passes, offscreen rigs,
frame tests).

### 4.2 Ownership: the same question, twice

| question | first owner | second owner | who wins |
|---|---|---|---|
| is this point shadowed by the beam? | `CellField` → per-corner `light_direct` (scene lattice) | `SunShadow` cascades → `sun_visibility` (`shading.wgsl:1365`) | the map, whenever shadows are on |
| how bright is the sky at a measured corner? | `skylight::SkyModel` (rig ambient, `shading.wgsl:1345`) | the field's `light_diffuse` (`shading.wgsl:1396`) | **both are added** — PATCH-063's open defect #1 |
| where a ray crosses matter? | `voxel-mesh::raycast::Spans` / `Crossings` | `voxel-stream::light`'s trees and stepped walk | one walk, two consumers; the tree path is not medium-aware |
| where is the Sun/Moon? | `voxel_shapers::sky` | `sun.rs` bodies, `SunShadow` direction, `WorldLight` beam | one source, four derivations — correctly threaded since PATCH-066's epoch |
| what is a cell? | `voxel_model::subvoxel` (4³) | `FILL_SIDE = 32`, `DETAIL_SIDE = 16` | three lattices, each named; no refusal if a consumer confuses them |

The first two rows are the audit's structural finding: the beam answer is split and silently
resolved in a shader branch, and the sky answer is summed twice. The corpus knows both (PATCH-063
named the second; PATCH-070 named the first as a past defect) but the tree still carries them.

### 4.3 What is genuinely one owner

- The lunar epoch: `GAME_NEW_MOON_TICK` is one value, threaded through the boot, the cadence and the
  look-ahead (PATCH-066's real contribution).
- The exposure: `EXPOSURE_GRADE = 1.0` is one constant, and the test proves the grade does not move.
- The subvoxel span: `Spans::split` is now the only splitter, and `march_tree` spends its answer.
- The cell-shell schema: `HOLO` is closed, and `retint` is the only path from a paint to a colour.

These are the parts a reader can hold in one hand. They are why the ownership grade is C and not
lower: the split is in the *lighting* head, not everywhere.

---

## 5. Ranked gaps

### P0 — before the next claim is written

1. **Correct PATCH-073's scope, or make it true.** The example measures with the beam shadow off;
   the binary draws with it on. Either call `set_shadows(true)` in `shipped` and re-measure the
   shoulder through the map, or retitle the record to the shadows-off frame and say what the lattice
   contributes to the picture (the sky term, and the direct band only where the rig has no light).
   The same correction belongs in PATCH-070's "the frame asks at subvoxel resolution".
2. **Choose the shadow's owner.** The lattice and the cascades answer one question and the shader
   discards one answer. The smallest honest change is to make the branch explicit: either the map
   wins (and the lattice's direct band is documented as measurement-only, with its cost justified by
   the sky term), or the field wins inside its measured range (and the map is the fallback for
   unmeasured corners). Paying for both and spending one is the current state.
3. **Commit the tree.** 307 modified, 676 untracked, 3 deleted, 18 817 insertions in tracked files,
   and HEAD is `BACKUP pre-flicker-rewrite`. No patch record can be tied to a commit, no rollback can
   be tested against the state it names, and the axis crates (seven directories) are not in git at
   all. This is the precondition for every other finding being reviewable.

### P1 — correctness of the record, cheap to fix

4. **Reconcile the three corner-arena figures** (`163.4 / 168 / 178 MB`) and the `128 MiB` doc line.
   State the bound and the observed peak as two numbers, with the live count each is relative to.
5. **Re-stamp or scope the stale ticks** in PATCH-066 and PATCH-073 (9 000 → 81 000; 24 000 → 216 000
   ticks a day), as PATCH-066 already did for PATCH-068.
6. **Mark PATCH-069 superseded by PATCH-070**, and repair its rollback; repair PATCH-072's
   `ARENA_SLACK` rollback.
7. **Fix the beam-step witness.** `the_frame_is_exposed_once_at_every_hour_and_never_lifted` prints
   `sky.sun` (`8.4051 → 8.3988`) and calls it a step; the rig's `lights[0].intensity`, which is the
   quantity that switches, is not printed, and `assert!(light_step > 0.0)` passes on a 0.075 %
   change. Print the rig's intensity and assert the step the defect is about, or drop the finding.
8. **Stop the scratch leak.** One `remove_dir_all` in `a_tampered_artifact_is_refused_before_it_is_shown`,
   one `.gitignore` line, delete 390 directories.
9. **Make the build quiet** (or at least make one crate's warnings visible). 2 328 warnings is not a
   policy; it is a place where the next real warning will not be seen.

### P2 — structure and latent defects

10. **Give `fog_cubes` a call site or take it out of `pub mod`.** A finished module with two tests and
    no frame is a claim without a picture.
11. **Delete the dead `visibility` factor** or lift the `sun_is_up` gate so it means what PATCH-065
    says. If the gate stays, the record should say the beam still switches at the horizon.
12. **Fix the sky double-add** (PATCH-063's open #1): the ambient and the field's diffuse sky are
    summed on every measured corner. The record says it wants a measurement of the two terms; the
    measurement is `the_worlds_light_across_a_sunset`'s crossing ratio, already printed.
13. **Reconcile the geometry step.** `GEOMETRY_STEPS = 2.0` (`main.rs:2595`) rebuilds geometry at
    1.0° while the field re-marches at 0.5°; a shadow can lag a degree. This is named in PATCH-063
    and unchanged.
14. **Make `march_tree` medium-aware, or refuse a medium there.** Today every tree provider returns
    `None` from `Matter::medium`, so nothing is wrong; the trait permits a provider that would be
    silently unattenuated. A refusal is cheaper than a future bug.
15. **Finish PATCH-071's owed doc edit** at `cells.rs:356` ("eleven" → "twelve").

### P3 — hygiene

16. The 402 MB `shploingus-console-17360-file-prose` scratch; the twenty ignored tests' visibility;
    the `crates.zip` path in the repository summary (no file at that name in the tree; confirm it is
    not expected).

---

## 6. What this audit did not do

- It did not run the shipped binary for five minutes, so the live-corner and frame-statistics claims
  of PATCH-072 are quoted.
- It did not compare shell frames; PATCH-071's table of screen losses is taken as prose.
- It did not audit the axis crates, the package loader, the plan's M10 missions, the LOD material
  work, or the clean-room. Those are a different head.
- It did not decide whether the lattice's cost is worth its sky term. That is a product decision, and
  the numbers for it are now in one place (§2.2).

---

## 7. The one-paragraph version

The corpus is unusually honest: the shoulder reproduces to the pixel, the night share reproduces to
0.04 %, the retint arithmetic and the arena bound are right, and the open defects are written down.
What it lacks is a single owner for the beam's shadow: the lattice is paid for, measured in a frame
that has the map switched off, and then overwritten by the map in the frame the player sees. Fix that
branch, re-scope PATCH-073, commit the tree, and the rest of this list is bookkeeping.
