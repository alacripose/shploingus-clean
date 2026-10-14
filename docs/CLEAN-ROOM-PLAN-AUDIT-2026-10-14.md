# The clean-room plan audited against the parent's own bytes — 2026-10-14

**What this is.** An audit of the artifact that tells the clean room what to build — `docs/CLEAN-ROOM-PLAN-2026-10-11.md`
and the seam documents it has spawned under `docs/baselines/port-appearance-seam/` — checked against the parent's
**current** bytes (`crates/shploingus/src/main.rs`, `crates/voxel-client/`, `crates/voxel-stream/`) and against the
clean room's own code. No source, test, plan or record line was changed by this pass: this file is the whole of it.

**The authority this audit reasons from**, because it outranks the plan: the user's ruling that the **cubic chunks format
is the cause** of the material flicker in the WGSL path, and that the reason it was never fixed is that **the work to
move away from it was never completed**. The plan carries the same ruling at lines 3999-4013 (quoted verbatim), and
`patches/PATCH-040-the-column-world-against-the-cubic-chunk-format.md` opens with both user directions.

**Exercised while auditing** (not re-read): `cargo test --offline -j 2` -> exit 0, **456 passed / 0 failed** over 18
suites; `cargo clippy --offline --all-targets -- -D warnings` -> exit 0; `cargo run --offline` -> exit 0, **532**
stdout lines, `Clean room OK`; `cargo run -- -- window 200 <ppm>` -> exit 0, 200 frames, read-back
**66 355 200 of 66 355 200** pixel(s), frame digest `7816bf6a…`; and the shell path driven by hand:

```
command: printf 'where\nup 4\nwhere\nforward 3\nwhere\nquit\n' | cargo run --offline -j 2 --quiet -- drive
drive: the person's own eye (32.00, 62.92, 185.90) m, yaw 180.0°, pitch -75.0°, over chunk (1, 1, 2) …
drive: 4 step(s) of Wish { forward: 0.0, strafe: 0.0, rise: 1.0, turn: 0.0, tilt: 0.0 }: the eye
       (32.00, 62.92, 185.90) m -> (32.00, 60.60, 186.52) m (2.40 m moved, 0.0° turned) …
drive: the person's own eye (32.00, 60.60, 186.52) m, yaw 180.0°, pitch -75.0°, over chunk (1, 1, 2) …
```

Four steps of "up" raise the eye 2.40 m and it **stays there**: there is no gravity, no ground and no collision
anywhere in the client's step. That is the audit's first finding, measured rather than read.

**What this audit did not reproduce, and says so:** the flicker itself. The clean room contains no shader (plan
`:4042-4043` says the same and refuses to pretend otherwise), and the parent's WGSL path needs a GPU build of
`crates/voxel-client` outside this pass's remit; so the **cause** (the cubic chunks format) is carried here as the
ruling's authority plus the record's own lines (`plan:3999-4013`, `PATCH-040`, `LEDGER.md:13510-13518`), not as a
measurement of this pass's own.

---

## 1. The three complaints, settled from bytes

### 1.1 The camera by default is flight — and it is the parent's **dead** path (stale import + regression)

| what | where |
|---|---|
| the parent's live camera is a **body** | `crates/shploingus/src/main.rs:2274` `self.walk_player(dt);` -> `:3034 fn walk_player` -> `:3050 self.player.advance(&wish, self.camera.yaw_rad, dt, &world);` `:3051 self.camera.position_m = self.player.interpolated_eye();` — the eye is **derived from the body** every frame (`:653-656`: *"The player's body, which **owns** the camera's position … the eye can never be somewhere the body is not"*) |
| the parent's movement is Squake | `crates/shploingus/src/main.rs:436` *"moving is the Squake model; jumping while sneaking with speed trumps — trimping, and it stays on"*; `crates/voxel-client/src/motion.rs:101 WALK_SPEED = 4.317`, `:115 SNEAK_SPEED = 2.2`, `:120 JUMP_SPEED = 8.4`, `:94 MAX_DT = 0.25`, `:454 pub fn advance(&mut self, wish: &Wish, yaw_rad: f64, dt_s: f64, world: &impl Solidity)` |
| the parent's `Wish` is a **body's** | `crates/voxel-client/src/motion.rs:295-298` (*"in intent rather than in keys … what stops walking forward-and-right being 41 % faster"*), `:298-310` `forward, strafe, sneak, sprint, jump` |
| the parent's **free flight is dead code** | `crates/shploingus/src/main.rs:143 const FLY_M_PER_S: f64 = 18.0;` is used at **exactly one** line, `:3076`, inside `laya_observation` (`:3055` *"**The floating observer, kept for the Laya god and no longer the player's** … **Nothing calls this, and that is deliberate rather than forgotten.**"*) |
| the clean room took the dead path | `clean-room/src/viewpoint.rs:80-84` `FLY_M_PER_FRAME = 18.0 / 30.0` (*"The parent's own free-flight speed"*), module doc `viewpoint.rs:72-74` (*"This is a free-flying eye, which is what the parent's own client is"*, quoting the bin's **stale** doc line `main.rs:82` *"No physics. Moving is free flight"*), `Viewpoint::step` = move + turn + tilt with no gravity, no ground, no collision; `clean-room/src/controller.rs:219 WALK_STEPS`, `:231 check_u1_controller` ("walks" 8 steps at 0.60 m); `clean-room/src/surface.rs:1005 step_driven(&plane, &mut residency, &mut viewpoint, &window.wish())`, keys at `surface.rs:1502-1518` and their held table at `:1990-2004` |

**What it is: a stale import of a parent path the parent itself no longer uses, and a regression of the clean room's
own design.** The intent vocabulary the clean room ported (`Wish { forward, strafe, rise, turn, tilt }`) is *flight's*
— the parent's live `Wish` is the body's — so the port is not a port of the parent's controller at all: it is a port
of the framework the parent retired, with the retired path's speed (18 m/s, ~4× the parent's `WALK_SPEED`) and none
of the parent's collision, ground or gravity. `resquake` — the Squake body — is assigned **no row** in the clean room
(plan lines 2011-2068), which is the plan's own instruction not to port it; that instruction is what makes the
mission's first named capability (player controller) impossible to satisfy here.

**Why it makes the clean room harder than the parent:** every system built after the camera (beings walking, water
you can stand in, terrain you can collide with, the acts of U12/U10) needs a body under it, and the clean room has
none; when the body is finally ported, the controller's whole vocabulary moves with it, so the cost of this import
grows with every pass that follows it.

**Not a genuine blocker.** The two files the parent's controller lives in read no pixel; the blocker is that they
live in a crate (`voxel-client`) that pulls `wgpu`/`glyphon`/`pollster` (`crates/voxel-client/Cargo.toml:11-25`, the
reasons are written out there). The smallest step that removes it is the split already named in
`docs/baselines/port-appearance-seam/parent-blocker-2026-10-14e.txt`: `motion.rs` (1 676 lines) + `camera.rs` (470
lines) into a crate with no GPU dependency, declared as the path dependency.

### 1.2 The terrain was not cubic chunks — the ruling landed in the **cells** but not in the paths that read materials (unfinished migration)

| what | where |
|---|---|
| the clean room's cells **are** cubic now | `clean-room/src/chunk.rs` (`32³` chunks, `Cell { material: Option<MaterialId>, water_um }`, air named), `clean-room/src/surface.rs:1128-1138 client_world() -> CubicWorld::from_fields(&roster, identity, Tick(0), request{origin, side: CLIENT_SIDE}, &ladder, 1)`, the law steps it (`automaton`), the being reads it (`being::WorldRead::over_cubic`) |
| but the drawn path ignores the ladder | `clean-room/src/drawn.rs:1024 crate::mesh::Level::new(0)` — the drawn reading is **level 0 only**, so `mesh.rs`'s ladder/refine machinery (2 876 lines) draws nothing at any other level |
| the **column world is still the shapers' world** | `clean-room/src/stage.rs:297 pub struct Patch { origin: VoxelPos, side: u32 }` (*"A patch of `side × side` **columns**"*), `clean-room/src/process.rs:32,435` (a `Patch` in the declared request), `clean-room/src/shaper.rs` + `shapers/{terrain,life,water,record}.rs` (every shaper is handed a `Patch`), `clean-room/src/world.rs`, `clean-room/src/field.rs` |
| and the **appearance path reads a column surface** | `clean-room/src/presentation.rs:87-95 pub struct Surface { reflectance, roughness, metalness }`, `:167-172 pub fn shade(surface: &Surface, normal: [f64; 3], light: &Light, view: &View) -> Result<Rgb, RefusalKind>` — one material record, one normal, one view: **no cell, no face, no footprint, no distance** |
| **the parent has already deleted its own column module, and nothing calls it** | `crates/voxel-shapers/src/column.rs` — **absent** from the working tree (`git diff --stat` -> *1 file changed, 1203 deletions(-)*; `ls` -> *No such file or directory*), `crates/voxel-shapers/src/lib.rs` names no `column`, and `grep -rn 'column::' crates/*/src` -> **no matches**: the parent's current bytes have no column-format module to import |
| the plan's own record of the cause | plan `:3994-4013` (every owed list in §35/§36 describes a column world; the user's ruling verbatim *"the cubic chunks format caused massive rendering bugs … it is the reason why there was materials flickering in wgsl"*), plan `:4024-4028`, and `docs/LEDGER.md:13510-13518` (C-189: *"a per-column material read is what reached a shader and produced the material flicker in the WGSL path, which is why this branch exists"*) |

**What it is: a regression of the clean room's design plus an unfinished migration.** The ruling is in the plan, the
cause is in the plan, and the *client's cells* comply — but the material path that the cause names **still reads a
per-column surface into the one shading call** (`presentation.rs:167`), and the shapers that produce materials still
take a `Patch`. So "what material is here" has two owners (`chunk.rs`'s cells and `stage.rs`'s columns), and the
appearance path reads the one the ruling refused. The flicker's cause is therefore still in the tree, one layer under
the client, and nothing in the plan makes its removal a row — `PATCH-040` froze U1's GPU pass pending the ruling, §37
unfroze it "as a shader over **voxel faces**", but the U1 row's bar is still the band count (plan `:329`).

### 1.3 No material-LOD rendering — the parent's eye-checked ladder is pinned, read, and never ported (seam-spec hole)

| what | where |
|---|---|
| the parent has it, and it is the *only* place the user's own eye was checked for weeks | `crates/voxel-client/src/pattern.rs`, 1 208 lines: the octave ladder (`15.6 mm · 2^j`, `OCTAVE_FLOOR..=OCTAVE_TOP`, gains/normalisation, `eye_footprint_m`, `octave_weight`), live in `crates/voxel-client/src/shading.wgsl:539,602,635`; the ledger's C-161…C-171 and `examples/pattern-flicker.rs` are that work (`docs/LEDGER.md:11860`, `:11980`, `:12205` *"**Named, not diagnosed:** the user reports the material pattern still flickering when the eye moves"*, `:12209`) |
| the parent's mesh has **no** LOD switch — so material LOD is not an alternative to a mesh ladder, it is the feature | `crates/voxel-mesh/src/lib.rs:24` (*"greedy merging cannot tell a 128×128 flat plane of one material from a plane of one material … with **no level-of-detail switch** and therefore no seam where the two paths meet"*) |
| the clean room has none | no `pattern`/octave/footprint anywhere in `clean-room/src/presentation.rs` or `drawn.rs`; the shading seam (`presentation.rs:167`) cannot express one (no footprint term); the U1 bar is `clean-room/src/presentation.rs:147 BAR_MINIMUM_BANDS = 3`, and the plan's U1 row asks for exactly that — *"the band count and a screenshot"* (`plan:329`) |
| the port's own contract never mentions it | `grep -n -i 'lod\|pattern\|octave\|flicker' docs/APPEARANCE.md` -> **no matches** (the pinned appearance contract, 142 lines), while `crates/voxel-client/src/pattern.rs` **is** pinned in `docs/baselines/port-appearance-seam/pinned-parent-bytes.txt` (`8a0f36e9…`) — pinned, i.e. read, and then never ported |

**What it is: a genuine blocker to porting the good code, created by the plan's own seam.** The parent's LOD needs a
per-**fragment** footprint and a shader; the clean room's seam is one `shade` call per rectangle over a declared
`Surface`, with no WGSL anywhere by design (plan `:4042-4043` *"this tree contains no WGSL and no shader, so nothing
in it can reproduce the defect"*). So the clean room cannot exhibit the flicker, cannot reproduce it, and cannot fix
it: it renders a **3-band fill** where the parent renders an eye-checked material ladder. That is not surpassing the
parent — on the user's own complaint list it is *below* it, and the plan's acceptance bar (bands >= 3) is the reason
its own passes never noticed.

---

## 2. The offending directives, each classified

1. **"No physics. Moving is free flight"** (the bin's stale doc, `crates/shploingus/src/main.rs:82`, taken as the
   parent's current client at `clean-room/src/viewpoint.rs:72-74` and in
   `docs/baselines/port-appearance-seam/parent-blocker-2026-10-14e.txt` — this thread's own file, written by the
   previous pass) — **stale import.** The parent's live client is `walk_player`; the sentence describes a mode the
   parent deleted and kept only as the Laya vantage.
2. **"the parent's own free-flight speed"** (`clean-room/src/viewpoint.rs:80-84`) — **stale import.** `FLY_M_PER_S`
   is the Laya vantage's number (`main.rs:143` -> `:3076`); the parent player's speed is `WALK_SPEED = 4.317`
   (`motion.rs:101`).
3. **"the parent's own free-flight map"** (the U1 registry row printed by the run, `clean-room/src/systems.rs:64`;
   and the same claim in the window's key docs at `clean-room/src/surface.rs:1502-1518`) — **stale import.** The
   live map is the body's translation at `crates/shploingus/src/main.rs:3036-3039` (`forward: key(KeyW) - key(KeyS)`,
   `strafe: key(KeyD) - key(KeyA)`, `Space` jump, `ControlLeft` sneak).
4. **"resquake … assigned no row"** (plan `:2011-2068`) — **regression of the clean room's priorities, not a stale
   import.** It is a defensible refusal of *GPL-3.0 third-party content*, but the plan then also refused the parent
   **first-party** body that implements the same movement, so the mission's first capability has no route.
5. **The column world as the phase-1 shape of the whole tree** (`stage.rs:297`, `process.rs`, `shaper.rs`,
   `shapers/*`, `world.rs`, `field.rs`; built as U2/U3 and the base of U4/U5/U6/U7/U8/U9/U10/U12 per plan §5's rows)
   — **regression, and a stale import that is already out of date in the parent.** The parent is 3-D
   voxels/subvoxels (`crates/voxel-mesh/src/lib.rs:7-15`, `crates/voxel-stream/src/world_source.rs`), and it has
   **already deleted** the column module it once had: `crates/voxel-shapers/src/column.rs` is gone from the working
   tree (1 203 deletions), unreferenced anywhere. The clean room imported that shape as its phase-1 world, spent
   eight slices on it, ruled it refused for cause (§37) — and then left the *appearance* path and the shapers still
   reading it. The user's point is the whole finding: the move away from the cubic chunks format **is** the fix, the parent
   has taken it, and the clean room has not.
6. **U1's acceptance bar = band count + screenshot** (plan `:329`, `presentation.rs:147`) — **seam-spec hole.** It
   is satisfiable by a flat fill with three luminance steps and says nothing about materials, LOD, or the defect the
   branch exists for.
7. **The flicker's mechanism as an owed audit** (plan `:4042-4043`, `PATCH-040`) — **correctly identified, never
   run.** The plan says the mechanism is not established here and cannot be; the user has since ruled the cause
   (the cubic chunks format) and the fix (moving away from it). The plan has no row for either.

---

## 3. Where the plan makes the clean room harder to build on than the parent

* **Traversal.** The parent walks, jumps, collides and trims; the clean room's client hovers, provably (`drive`,
  section 1.1). Every gameplay system that follows needs the parent's body, and the clean room's controller
  vocabulary has to be rebuilt to accept it.
* **Materials.** The parent renders an eye-checked ladder that the user spent weeks on; the clean room renders a
  band-count fill whose bar (>= 3 bands) cannot tell the two apart. Porting the ladder later means replacing the
  seam, not extending it.
* **One world, or three.** The parent has one owner of "what is here" (voxels -> octree -> mesh). The clean room has
  cells (`chunk.rs`), columns (`stage.rs`'s `Patch`, `world.rs`, `field.rs`) and a region mesh (`mesh.rs`), with the
  shapers on columns and the law/client on cells — so a change to a material has to land in more than one place, and
  the appearance path reads the wrong one.
* **Dead LOD machinery.** `mesh.rs`'s ladder/refine (a large share of 2 876 lines) exists for levels the drawn path
  never asks for (`drawn.rs:1024`, level 0), while the LOD the user can see (`pattern.rs`) is absent — effort in the
  wrong LOD, none in the right one.
* **The record's own weight.** ~1.3 MB of dated seam documents under `docs/baselines/port-appearance-seam/` account
  for a client that cannot walk; `evidence.rs` alone is 5 459 lines and the plan's own §41/§42 accounts name the
  split as overdue for the eighth pass running.

---

## 4. The plan, graded

| dimension | grade | what carries it, and the top gaps (priority order) |
|---|---|---|
| **SPEC** — the request's own outcomes, present and working | **3/10** | Of the mission's three named capabilities, **one** is real behaviour: chunk streaming (`clean-room/src/stream.rs`, measured — 4 of 4 chunks over the shown state, 0 differing of 131 072 against the world's own door, first eviction at 45.2548 m, the window draws through the residency). **Top gaps:** (1) *player controller* — no body, no collision, no gravity: the camera flies at the retired path's 18 m/s (`viewpoint.rs:80`, proven by `drive`); (2) *material LOD rendering* — absent entirely, and the U1 bar (`presentation.rs:147`) cannot ask for it; (3) *the flicker's fix* — the plan names the cause (the cubic chunks format, plan `:3999-4013`, `LEDGER.md:13518`) and then makes no row of the cubic-addressed material read that would remove it. Also counted against it: the plan's rails are where it spends its instruction — `resquake` refused a row (plan `:2011-2068`) is what forecloses the parent's own body. |
| **DESIGN** — one owner per state; a change lands in one place | **4/10** | Good: the camera/player state has one owner (`viewpoint.rs` + `controller.rs`), and the residency has one (`stream.rs`). **Top gaps:** (1) **two world models** — `chunk.rs`'s cells vs `stage.rs:297`'s `Patch`/`world.rs`/`field.rs`, with `shapers/*` and `presentation.rs:167`'s `Surface` on the column side and the law, being and client on the cell side (this is also the flicker's cause, so it is a design defect with a rendering symptom); (2) **`surface.rs` (2 089 lines)** owns the client, the platform window, the world, the driven loop and the counters, and **`evidence.rs` (5 459 lines)** is the only home for every measurement — a maintainer cannot separate the concerns without reading both; (3) **`mesh.rs` (2 876 lines)** owns an unexercised LOD ladder (`drawn.rs:1024` draws level 0). |
| **CORRECTNESS** — behaviour actually exercised | **5/10** | Exercised this pass: the full gate is green (`456 passed / 0 failed`, 18 suites, exit 0), clippy clean, `cargo run --offline` exit 0 / 532 lines, the window presents 200 frames with every pixel read back (66 355 200 of 66 355 200) and writes the frame, and the controller's walk is a real measurement (8 steps west 4.80 m, anchor (1,1,2) -> (0,1,2), every frame's digest changes, and 8 steps east return the frame exactly: 1333 quads, 147 808 of 331 776 pixels, `7816bf6a…`). **Top gaps:** (1) the driven camera has no physics — 4 steps of "up" hold the eye 2.40 m higher forever (measured, section 1.1); (2) the defect the branch exists for is neither reproduced nor fixed, and cannot be in this tree (no shader; the cause's per-column read is still live at `presentation.rs:167`); (3) the LOD machinery is exercised by nothing — level 0 is the only level any drawn frame uses. |
| **QUALITY** — concision against the smallest version of the same behaviour | **3/10** | **Top gaps:** (1) the column world and its pipeline (`stage.rs`'s `Patch`, `field.rs`, `world.rs`, the shapers' column reads, plus the column-shaped tests) exist to satisfy a phase-1 shape the plan itself later refused — the single largest deletion opportunity in the tree; (2) `evidence.rs` 5 459 lines carrying every pass's measurements in one file, growing for the eighth pass running (the plan's own §41 account says the split is overdue); (3) the LOD ladder in `mesh.rs` and the seam documents' ~1.3 MB of dated captures describe capabilities the client cannot use, so the same behaviour (a driving, streaming client) is wrapped in several times its own weight. |

**The single most valuable next pass: finish the migration the ruling demands.** One pass that (a) makes the material
path read the **cubic cells** — a per-cell/per-face material read replacing `presentation::shade`'s per-column
`Surface` — and (b) puts the parent's **body** on those same cells (port `crates/voxel-client/src/motion.rs`'s
`Player::advance`/`WALK_SPEED`/collision and derive the camera from the body, which is the parent's own rule at
`crates/shploingus/src/main.rs:653-656`), with the flicker re-measured in the parent's own path to confirm the column
format was the cause. It is the only pass that turns a row into a capability *and* attacks the defect this branch
exists for; the LOD ladder (a per-fragment footprint, i.e. `pattern.rs`'s seam) follows in the same shape once the
material read is cell-addressed, and neither can be done while `Wish` still means flight and `Surface` still means
column.

---

*This audit changed no source, test, plan, pin or capture. The two seam documents written by the previous pass in this
thread (`parent-blocker-2026-10-14e.txt`, `guard-bites-2026-10-14e.txt`) and the run capture beside them are named
above where they repeat the stale free-flight claim rather than corrected here, because the correction belongs to the
pass that fixes the camera.*
