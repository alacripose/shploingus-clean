Port evidence pass, 2026-10-13 (§43, PATCH-041's first step). Evidence only: no clean-room module replaced, no seam
ported, nothing deleted, no dependency added, nothing under `crates/` edited. `crates/` was read and *built*, not
touched; every parent figure below is pinned by **bytes**, never by branch name, because that tree is dirty (123
modified paths under `crates/` at the time of writing).

toolchain: rustc 1.99.0 (b940084d7 2026-09-28) · cargo 1.99.0 (5f94df478 2026-08-27)
workspace: C:\Users\ala\Documents\shploingus-clean (branch main, HEAD 61e1223dc7f6ae28bc7da24b926c9b83aa333306db1e55be9ecbdec120eb5d05)

--------------------------------------------------------------------------------
1. The offline resolve: one crate, named exactly
--------------------------------------------------------------------------------

  cargo metadata --offline --format-version 1
    → exit 101
      error: failed to download `wasi v0.11.1+wasi-snapshot-preview1`
      Caused by: attempting to make an HTTP request, but --offline was specified

  cargo metadata --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
    → exit 0    (the whole graph, host-filtered, resolves offline)

**What is missing, and where it would have to come from.** `wasi v0.11.1+wasi-snapshot-preview1`, checksum
`ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b`, source
`registry+https://github.com/rust-lang/crates.io-index` (`Cargo.lock` line 3252). It is pulled by `getrandom 0.2.17`
(`Cargo.lock` line 864, `dependencies = ["cfg-if", "libc", "wasi"]`) — a **wasm-target-only** dependency. The local
cache holds the *newer* wasm shim (`~/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/wasip2-1.0.4+wasi-0.2.12.crate`,
`getrandom 0.3.4`'s) but not this one. It would have to arrive as
`~/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/wasi-0.11.1+wasi-snapshot-preview1.crate` — one
`cargo fetch` (a single HTTP request for a ~100 KB crate), or a copy from the kache store at `C:/kache`, or any other
machine's cargo cache.

**Consequence, measured and bounded:** the gap is *not* a build blocker on this host. Every host build and run below
succeeded **offline**; only whole-graph operations (`cargo metadata`, `cargo fetch`) and a `wasm32-*` target need it.
This claim is checked by three runs in this directory, not asserted: see §2 and §3.

--------------------------------------------------------------------------------
2. The game crate's build state, measured
--------------------------------------------------------------------------------

  cargo check --offline -p shploingus
    → exit 0 in 3 m 15 s (dev profile, optimized + debuginfo); 0 errors, 2277 warning lines
    full log: shploingus-check.log

`crates/shploingus` — the parent's game binary — **checks offline today**. Its own warning (1 in the crate itself,
`unused_mut` in `build.rs`-adjacent code) is in the log. This was the number the record left unmeasured in PATCH-041.

--------------------------------------------------------------------------------
3. The parent's own flicker instrument, measured — twice, on two byte-pins
--------------------------------------------------------------------------------

The instrument is `crates/voxel-client/examples/pattern-flicker.rs` (its head records the user's 2026-10-08 report:
*"still visible material flickering when moving on the vertical axis which is now fixed when moving on the horizontal
axis"*). It draws the same world from two eyes a stated step apart, once vertically and once horizontally, and prints
the per-pixel difference in bands, with a **camera-held control** that must read zero.

  (a) The prebuilt binary — bytes of 2026-10-11 18:25, i.e. the *previous* state of that tree:
      ./target/release/examples/pattern-flicker.exe    → exit 0
      capture: pattern-flicker-prebuilt-2026-10-11.txt
      0.05 m: vertical (+y) 22340 of 230400 px (9.7 %), total 360.2, mean 0.001564, worst 0.8695
              horizontal (+x) 6662 (2.9 %), total 145.4, mean 0.000631, worst 1.0658      → 3.35× / 2.48×
      0.25 m: vertical 49494 (21.5 %), total 1616.4  | horizontal 21888 (9.5 %), total 706.4
      1.00 m: vertical 72159 (31.3 %), total 4798.5  | horizontal 65976 (28.6 %), total 2511.8
      both camera-held controls: 0 of 230400 changed, total 0.0

  (b) The current (dirty) sources, rebuilt and run:
      cargo run --offline --release -p voxel-client --example pattern-flicker   → exit 0
      built in 33.20 s (release), ran immediately; capture: pattern-flicker-rebuilt.txt, build log:
      pattern-flicker-rebuilt.stderr.txt; binary pinned in instrument-binary.txt
      pattern-flicker: 18 chunks, 5622 quads, meshed in 1456 ms; eye (48.00, 32.00, -24.00),
                       0.003890 rad a pixel, step 0.05 m
      0.05 m: vertical (+y) 10966 of 230400 px (4.8 %), total 332.5, mean 0.001443, worst 0.8667
              horizontal (+x) 3893 (1.7 %), total 140.0, mean 0.000608, worst 1.0816      → 2.82× / 2.38×
      0.25 m: vertical 34318 (14.9 %), total 1566.5  | horizontal 14658 (6.4 %), total 689.8   → 2.34× / 2.27×
      1.00 m: vertical 69689 (30.2 %), total 4773.1  | horizontal 46048 (20.0 %), total 2456.9  → 1.51× / 1.94×
      both camera-held controls: 0 of 230400 changed, total 0.0

**The gate number the appearance port must meet.** On the current bytes a 0.05 m vertical step changes **2.82×** as
many pixels as the same-length horizontal step (2.38× by total magnitude), and the asymmetry is still present at every
step size. A pattern that merely scrolls with the geometry changes by one pixel of pattern per pixel of geometry
moved; this excess *is* the re-weighting the user reported. The port's gate is therefore: **vertical ≈ horizontal at
equal step**, with the camera-held controls still reading exactly 0.

**The environment, named rather than assumed.** The instrument *does* run here: no window, no display and no GPU
adapter was needed for the example's path, and both runs exited 0 offscreen (the prebuilt run's stderr is empty:
0 bytes). Numbers (b) belong to the current dirty tree, which is why the parent bytes are pinned below.

**Provenance limitation, stated rather than blurred.** Capture (a) is the stdout of the binary as it stood at
2026-10-11 18:25, and that binary has since been **overwritten at the same path** by the rebuild that produced (b)
(now 2026-10-13 14:07, sha256 `a78acac8a303c28c2dbc775f06554c2ca211ccb47aa9efd5a6d62bb98f0cf869`, 7 585 280 bytes —
`instrument-binary.txt`). Its own digest was **not** captured before the overwrite, so (a) is attributable by its
mtime and by the header line its own stdout prints (*"this run measures the shader as compiled"*) and not by a hash.
The `Cargo.lock` and crate digests in `pinned-parent-bytes.txt` are what let a later run be compared to either.

--------------------------------------------------------------------------------
4. The parent bytes the appearance seam reads, pinned by path + sha256
--------------------------------------------------------------------------------

  pinned-parent-bytes.txt — 71 lines: every `*.rs`, `*.toml` and `*.wgsl` under `crates/voxel-appearance` (11 .rs) and
  `crates/voxel-client` (49 .rs, the WGSL shaders among them), plus `docs/APPEARANCE.md` (the user's own document, the
  contract) and `Cargo.lock` (what the resolve above was measured against). Verify from the repository root with:

      sha256sum -c docs/baselines/port-appearance-seam/pinned-parent-bytes.txt

These are the digests `systems.rs`'s appearance row should later carry as `pinned_by`, beside the port's gate — the
row that today says `must_not_copy: "any file under crates/voxel-appearance or crates/voxel-client"`.

--------------------------------------------------------------------------------
5. What this pass did not do
--------------------------------------------------------------------------------

No clean-room module was replaced, no seam was ported, nothing was deleted, no dependency was added, and no file under
`crates/` was edited or removed (`find crates -newermt '-90 minutes' -type f` → 0; the only writes outside this
directory are `target/` build artifacts and the dated note in
`patches/PATCH-041-the-reconciliation-with-the-parent-branch.md`).

The clean room's final bytes are green: `cargo test --offline` in `clean-room` → **426 tests, 0 failures, exit 0**
(334 unit + 92 across fourteen integration suites), with no `.rs` under `clean-room/` having moved.

--------------------------------------------------------------------------------
6. The port itself — 2026-10-13, the same day, in the same directory
--------------------------------------------------------------------------------

The evidence pass above was step 1. This is step 2: **the seam is ported**, under Constitution P7 as amended
to v2.0.0 (the `path` dependency *is* the port, and the bytes it read are pinned by path + digest). Nothing
under `crates/` was edited, and the pin above still verifies: `sha256sum -c pinned-parent-bytes.txt` → exit 0,
71 of 71 OK.

  * `clean-room/Cargo.toml` — one declared addition, two lines: `voxel-appearance` (the appearance and frame
    code itself) and `voxel-noise` (the `Vec3` the ported `SurfaceFrame`/`Light` are handed their directions
    in, the same type the parent's own `npr` imports).
  * `clean-room/src/presentation.rs` — `shade`'s reduced `Surface`/`Light`/`View` are composed into the
    parent's `OpenPbrInputs`/`SurfaceFrame`/`LightRig`, and the colour returned is **the parent's**
    `voxel_appearance::shade`. The clean room's four re-derived stylisation terms and their four constants
    are **gone** — one owner for the appearance, not two. The sealed scene's `open` fraction is 0.0 and
    stated; the hemisphere fill is the parent's own declared rig ambient; the key light is this client's own
    argument, so the ported path honours the light it is given.
  * `clean-room/src/presentation.rs` + `surface.rs` + `drawn.rs` — a fifth refusal, named:
    `presentation::RefusalKind::AppearanceRefused` (`"appearance_refused"`), carried through both readings
    as this client's parameter fault with the ported contract's own kind in the detail. A roughness the
    parent's `validate` refuses is refused rather than clamped — the test for it is in `presentation`'s own
    module.
  * `clean-room/src/systems.rs` — the retired `must_not_copy` field is replaced by `pinned_by`: U1's row now
    carries the 71-path pin above, and the twelve rows with no ported seam say so in their own words.
  * `clean-room/tests/u1_port_flicker.rs` — **the gate, executable**, run against the ported path and
    captured in `port-flicker-2026-10-13.txt` beside the parent's own figures.

What the port changed, measured before (the immediately preceding baseline,
`docs/baselines/u4-the-stitched-window/final-stdout.txt`) and after (this tree's bytes, 2026-10-13):

  quantity                                     before the port      after the port
  U1 bar, 181-sample sweep fixture             14 luminance bands   19 luminance bands
  U1 client: plan reading, 576 x 576           20 bands             23 bands
  U1 client: plan frame digest                 bbdf3b914c97888e…    0933f12a08a5a02a…
  U1 client: surface read digest               70a06acd49f452bc…    70a06acd49f452bc…   (unchanged: same world)
  U4 drawn: rectangles / vertices / triangles  1333 / 5332 / 2666   1333 / 5332 / 2666  (unchanged: same geometry)
  U4 drawn: pixels drawn                       147808 of 331776 (44.6%)  147808 of 331776 (44.6%)  (unchanged)
  U4 drawn: luminance bands                    42                   47
  U4 drawn: frame digest                       4769858992491c11…    7816bf6a07dada5a…
  U4 drawn: mesh digest                        24d61db2630f77aa…    24d61db2630f77aa…   (unchanged)
  flicker: 0.05 m vertical vs horizontal       parent 2.82x / 2.38x ported 1.44x / 1.60x
  flicker: 0.25 m                              parent 2.34x / 2.27x ported 1.21x / 1.20x
  flicker: 1.00 m                              parent 1.51x / 1.94x ported 1.22x / 1.21x
  flicker: appearance asymmetry (the gate)     parent: not separable  ported 0.07x / 0.05x / 0.04x
  camera-held controls                         parent 0 of 230400     ported 0 of 331776

The geometry digests are unmoved and the pixel digests are not: the port changed what the world **looks
like** and nothing about what it **is**. That is the shape a correct port of the appearance seam has.

**The gate's first version was falsified by its own guard-bite experiment**, and that is the most useful
thing this pass measured. The gate was first stated on changed pixels (vertical against horizontal, against
the parent's pinned ratios). Injecting a **view-dependent gain** (up to a half) into the ported colour then
left the changed-pixel counts **identical to the digit** — 1837 and 1280 at 0.05 m — because a colour that
changes *inside* a pixel class does not change the class. A changed-pixel count is blind to exactly the
re-weighting the instrument exists to catch. The gate was therefore restated on the **appearance response**
(the frames' own shaded colours, summed over the mesh's 1333 rectangles): the vertical step must move them at
most half as far as the horizontal step does. On the ported bytes that reads 0.07x / 0.05x / 0.04x; on the
perturbed bytes it reads 0.96x / 1.00x / 0.96x and the gate fails by name. The ported bytes were restored
byte-exact afterwards (digests in `port-flicker-2026-10-13.txt`).

This pass does **not** replace the parent's own instrument, and does not claim to: that one measures the
parent's GPU/WGSL path, where the user's report came from. What this gate holds is the ported *shade* seam in
the clean room, at the same three steps, with the parent's figures printed beside every line.
--------------------------------------------------------------------------------
7. The mesh and stream seam — 2026-10-13, same directory, the same dirty tree
--------------------------------------------------------------------------------

§6 ported the **appearance**. This pass takes the record's next seam — the parent's mesh and stream code —
lands half of it measured, and names the other half's blocker in the parent's own signature. Nothing under
`crates/` was edited; the pin was **appended** (lines 72..87) rather than rewritten, because the same dirty
parent tree is the source, and it verifies: `sha256sum -c pinned-parent-bytes.txt` → **87 of 87 OK**. This
section is appended for the same reason. No head was rewritten anywhere.

  * `pinned-parent-bytes.txt` — 71 → **87** lines, +16 paths; the file's first 71 lines are unchanged.
    **Read by the ported call** (6 + 5 paths): `crates/voxel-mesh/{Cargo.toml,src/chunk.rs,src/lib.rs,
    src/mesh.rs,src/octree.rs,src/source.rs}` — the mesher, its two traits (`VoxelSource`,
    `ChunkNeighborhood`) and the octree — and `crates/voxel-model/src/{coord.rs,lib.rs,material.rs,
    refusal.rs,subvoxel.rs}` — the types that seam is spoken in (`ChunkPos`, `LocalPos`, `VoxelPos`,
    `CHUNK_SIDE`, `MaterialId`, `VoxelShape`, `SubvoxelIndex`, `Refusal`). **Read to establish the blocker,
    not called** (5 paths): `crates/voxel-stream/{Cargo.toml,src/lib.rs,src/resident.rs,src/horizons.rs,
    src/world_source.rs}`.
  * `clean-room/Cargo.toml` — **no dependency added by this pass.** `voxel-mesh` and `voxel-model` were
    declared in §6's own section of that file; `voxel-stream` was deliberately **not** added (the blocker,
    §7.4). No third-party crate was added either. The one edit here is a correction: that section's heading
    said *"the mesher and the residency"* while only the mesher is declared, which is exactly the kind of
    stale claim this tree keeps refusing, so it now names the mesher and states the residency's blocker.
  * `clean-room/src/ported_mesh.rs` — the bridge and the measurement (§7.1–§7.3), with two executable claims
    in it: the gate (`every_divergence_between_the_two_derivations_is_named`) and the stream figure.
  * `clean-room/src/systems.rs` — U4's row only: `pinned_by` moves from *"none yet"* to the appended pin, and
    `status` gains one dated clause naming what is bridged, what is not, and why. No other row moved.
  * captures, in this directory: `port-mesh-2026-10-13.txt` (258 lines — the port's whole stdout, the gate
    line on line 3) and `port-mesh-guard-bite-2026-10-13.txt` (the perturbation's own failure).

### 7.1 What the mesh bridge is

`ported_mesh.rs` hands this clean room's cells to the **parent's own `voxel_mesh::Mesher`** through the
parent's own two traits — `VoxelSource` for the chunk, `ChunkNeighborhood` for the ring the mesher reads one
voxel past each of the six faces — and compares the result with this crate's own derivation of the same chunk
of the same world. Three mappings are declared once, in its header, and two of them are the whole difference:

  1. **a material is its own number + 1** (`n ↦ n + 1`), so the parent's reserved `EMPTY` (0) stays reserved;
  2. **a cell is a cube** at the parent's 4×4×4 subvoxel resolution — and a wetted cell is handed over by one
     of **two declared readings** (`Water::FullCube`, or the bottom `k` of the parent's own four layers,
     `k` the cell's micrometre level rounded to the nearest quarter);
  3. **the axes are handed over unswapped** (×4 into subvoxels).

`Outside` is the run's third declaration — what the parent's halo is told where the world cannot answer:
`World` (the parent's own contract: refuse), `WorldOrNothing` (this crate's own chunk door's halo,
`MESH_RING`), `Nothing` (the parent's own `AllAir`). Every unanswered halo query is counted, never hidden.

### 7.2 The differential: every divergence named, with its number

The two meshes' **digests are not comparable and are not compared** — and that is a measurement, not a
disclaimer. The parent merges at subvoxel resolution and this crate at block resolution, so one face set has
two rectangle decompositions: on the client's own chunk the parent's 508 rectangles cover 4 043 unit faces
(**127.34** faces/quad) where this crate's 339 cover 2 830 (**8.35**); with the quarter-cell water reading the
parent's 336 cover 2 930 (**139.52**). What *is* comparable is the set of **unit voxel faces** each mesh
covers, and that is what is accounted for, by named reason.

  water handed over as **a full cube** — parent-only faces, by reason, and this-crate-only faces

  | world (chunks) | halo | parent only | the world's own bound | the declared halo said air | the water level quantised | clean room only |
  |---|---|---|---|---|---|---|
  | client (4) | `World` | **REFUSED** — *"[clean-room] (-1, 0, 64) is not in this world: its chunk (-1, 0, 2) is not one of the 4 this world owns"* | | | | |
  | client (4) | `WorldOrNothing` | 1 213 | 1 082 | 0 | 131 | 0 |
  | client (4) | `Nothing` | 1 939 | 1 082 | 726 | 131 | 0 |
  | fixture (9) | `World` | **REFUSED** — *"[clean-room] (32, 32, 63) is not in this world: its chunk (1, 1, 1) is not one of the 9 this world owns"* | | | | |
  | fixture (9) | `WorldOrNothing` | 1 212 | 1 024 | 0 | 188 | 0 |
  | fixture (9) | `Nothing` | 2 642 | 1 024 | 1 438 | 180 | 0 |

  water handed over as **the parent's own 1/4-cell resolution**

  | world (chunks) | halo | parent only | the world's own bound | the declared halo said air | the water level quantised | clean room only |
  |---|---|---|---|---|---|---|
  | client (4) | `WorldOrNothing` | 1 024 | 1 024 | 0 | 0 | 924 (all water) |
  | client (4) | `Nothing` | 1 690 | 1 024 | 666 | 0 | 924 (all water) |
  | fixture (9) | `WorldOrNothing` | 1 076 | 1 024 | 0 | 52 | 137 (all water) |
  | fixture (9) | `Nothing` | 2 488 | 1 024 | 1 412 | 52 | 137 (all water) |

  **The gate: 8 case(s) compared, 16 558 unit face(s) the world owns on the parent's side, 15 406 divergent
  face(s), and every one of them named — 0 unnamed.** Four of the twelve combinations are the parent's own
  refusal (`Outside::World`), which is the parent's contract answering correctly about a bounded world: there,
  **every chunk is a boundary chunk**, and the parent's neighbourhood says so. `WorldOrNothing` is the domain
  where the two derivations are the same subject — this crate's own chunk door's halo.

Three things this accounting measures, none of which a sentence would have carried:

  * **the water reading is a real choice with a real price.** A full cube makes the parent draw 131 / 188
    water faces this crate does not (it claims the water stands to the cell's ceiling); the quarter-cell
    reading makes this crate draw 924 / 137 the parent does not (the level is finer than a quarter of a cell,
    and the residual flips direction and grows). The two readings bracket the truth and neither is it: this
    world's water is a depth in micrometres, the parent's matter is 64 subvoxels. Both counts are printed.
  * **the parent's drawing of the world's own edge is not a defect in either mesher** — 1 082 (client) /
    1 024 (fixture) faces, the same number under both water readings and both halos, which is what a bound is.
    This crate's extent contract makes the world's own bound an edge; the parent's neighbourhood contract,
    handed air there, draws it.
  * **the `AllAir` reading costs exactly the faces the parent was lied to about** — 726 / 1 438 more, all on
    the meshed chunk's own six boundary planes, all culled by this crate against the neighbour the world
    really holds.

Subvoxel quantisation is disclosed rather than rounded: with the quarter-cell reading the parent emits 64
(fixture, `WorldOrNothing`) and 65 (fixture, `Nothing`) rectangles whose corners are **not** on a whole voxel;
`parent_block_faces` counts them and says so. Under a full cube of water it is 0.

### 7.3 Why nothing was replaced

The gate is **not equality**, and the bridge therefore replaces nothing. `agrees()` is false on all eight
comparable cases; what passes is that *every divergence is one of three named reasons*. `Unnamed` — a
disagreement about geometry both derivations can see — is 0 everywhere. "The same geometry" is not yet "the
same mesh": the rectangles differ (127.34 vs 8.35 faces/quad), the water reading is a declared choice, and the
world's edge is drawn differently by design. Per the pass's own constraint — *nothing in the clean room is
deleted before its replacement passes its gate* — nothing was deleted or replaced.

**The gate bites, and it was proved by perturbing its own subject.** The bridge's own cell→shape mapping was
perturbed for one run (a solid cell handed over as its **bottom half** only, so the two derivations disagree
about geometry the world owns). The guard's own failure, verbatim from
`port-mesh-guard-bite-2026-10-13.txt`:

  thread 'ported_mesh::tests::every_divergence_between_the_two_derivations_is_named' panicked at
  src\ported_mesh.rs:978:21:
  the client's own world, WorldOrNothing, water FullCube: the gate is not met — **21084 unnamed parent-only
  and 100 unnamed clean-room-only** of 23221 / 1906 divergent face(s); the accounting is total (the counts sum
  to 23221 of 23221 parent-only and 1906 of 1906 clean-room-only)

The accounting stays **total** under the perturbation — the counts still sum — so what fails is exactly the
property the gate exists for. The source was then restored **byte-exact**: `sha256sum -c` over a digest taken
before the perturbation → `src/ported_mesh.rs: OK`, identical before and after
(`08dd5fe54e94aa51c40afa6b7e5dbc6a363b91c13cb3e4683c13cfac2ea92749` at that moment).

**That digest is not the file's final one, and the difference is named rather than left to be noticed.**
After the bite experiment the module's three unused imports and its one test-only helper were tidied away (so
that `cargo build` reports **0** warnings from it), which moved its own bytes on to the figure recorded in
`digests-2026-10-13.txt` — `8e63b1d762de5681bf43c487edaf8d84982c8f3e0d0abb6ebb59716cff3056d2`, verified from
`clean-room/` with `sha256sum -c`. The three port tests pass on those final bytes, and on them the gate
reads 0 unnamed.

### 7.4 The stream seam: not landed, and the reason is the parent's own signature

The parent's residency is `voxel_stream::Resident` (`crates/voxel-stream/src/resident.rs`, pinned above). It
cannot be this clean room's residency, and the blocker is not a judgement:

      pub struct Resident { source: WorldSource, ... }        resident.rs
      pub fn new(source: WorldSource) -> Refused<Self>        resident.rs
      pub struct WorldSource { spec: WorldSpec, ... }         world_source.rs

A `Resident` streams a `WorldSource`, and a `WorldSource` **is** a `voxel_shapers::WorldSpec` — the parent's
generator. There is no trait, no generic parameter and no injection point between them, so the parent's
residency can only stream the parent's own generator. Two ways out, both outside this pass: **(a)** edit a
parent byte (forbidden — `crates/` is read and build only here), or **(b)** adopt the parent's generator,
which is U3's seam and not U4's. The dependency weight is the same fact:
`cargo tree --offline -p voxel-stream --depth 1` → `voxel-mesh`, `voxel-model`, **`voxel-shapers`** — three
direct dependencies where this port needs one, and 26 distinct crates in that tree.

The other half of the mismatch is the horizon's arithmetic. The parent's four declared horizons
(`crates/voxel-stream/src/horizons.rs`, `Horizons::declared()`) each name a **cube** of `(2r + 1)³` chunks
around a viewer, generated on demand:

      render      r =  8 chunks  →     4 913 chunks    this world owns 4 — 0.0814 %
      simulation  r = 12         →    15 625          this world owns 4 — 0.0256 %
      residency   r = 24         →   117 649          this world owns 4 — 0.0034 %
      field       r = 96         → 7 189 057          this world owns 4 — 0.0001 %

**The exact structural reason, in this world's own code.** `CubicWorld::from_surface` (`src/chunk.rs`)
creates every chunk of the patch's own footprint over the vertical span its surface's lowest and highest point
snap out to, then fills each column from the world's floor up to that column's ground — one finite **column
patch, materialised whole at startup**. `CubicWorld::cell` refuses every position whose chunk is not one of
those, in its own words, from the capture:

      stream-seam: this world's own answer above its ceiling: (0, 0, 96) is not in this world: its chunk
      (0, 0, 3) is not one of the 4 this world owns

So the answer to *"does geometry become chunk-addressable and streamable?"* is **addressable: yes** — the
chunks are whole (4 × 32³ = 131 072 cells for the client's world, 9 × 32³ = 294 912 for the fixture, 44 187 /
98 991 of them solid), the parent's own mesher runs on any of them, and the gate passes there — and
**streamable: no**, because the world is a slab that exists in full or not at all while the parent's residency
asks it for any chunk in a `(2r + 1)³` cube.

**The smallest next step that removes it: a chunk provider.** One door on `CubicWorld` — a chunk key in, that
chunk's cells out, derived from the same declared fields `from_surface` is handed today (`field::fields_at`
with `ProcessRequest::Region` already answers a region, and a chunk *is* a region) — makes the world a pure
function of (identity, tick, key) evaluated per chunk instead of a patch materialised whole. That is one new
door on the existing world, not a second world model, and it is the shape the parent's own
`WorldSource::build_chunk(pos)` has — which is what would make the parent's residency portable afterwards. The
port of `Resident` itself would still need its `source` field to be generic, and that is a parent edit this
pass's own constraint forbids.

### 7.5 What this pass did not touch

The readings' prose in its four homes, `evidence.rs` and its split, and every named owed item — the per-chunk
memory figure, a region's own residency rather than its one-footprint stitch, a strided field request, a
streamed residency policy, `mc-ar-mod`, the `noul`/`score` sets, the U10 rulings, PATCH-040's mechanism audit
— are **unmoved**. The flicker instrument's numbers stay the appearance seam's gate (§3, §6) and were **not**
re-measured here. `docs/CLEAN-ROOM-PLAN-2026-10-11.md` was read and not edited.

No parent byte was edited. Stronger than a timestamp: `sha256sum -c pinned-parent-bytes.txt` → 87 of 87 OK,
over bytes read before and after every run above. The weaker check, in the pattern §5 used:
`find crates -newermt '-120 minutes' -type f` → 0.

### 7.6 Done conditions, and one disclosed flake

`cargo test --offline -j 2` in `clean-room`, on the pass's final bytes → exit **0**, **430 passed / 0 failed**,
0 ignored, 18 suites, **9 733** lines of stdout, 0 error lines (9 734 on the earlier bounded run — the figure
moves by a line or two with the compiler's own progress lines) — 337 library tests (three of them this port's:
`the_parents_mesher_and_this_crates_derivation_are_compared_on_one_world`,
`every_divergence_between_the_two_derivations_is_named`,
`the_parents_streaming_and_this_worlds_shape_are_measured_against_each_other`) and 93 across the integration
suites. `cargo build --offline` → exit 0, with **0** warnings from `src/ported_mesh.rs` (its three unused
imports and its one test-only helper were moved where they belong rather than left warning).

The pass's own digests are in this directory, in the pattern §5 and the earlier baselines use:
`digests-2026-10-13.txt` — the three files this pass touched in the clean room (`src/ported_mesh.rs`,
`src/systems.rs`, and `Cargo.toml`, whose own comment now says that the residency is *not* declared and why),
verified from `clean-room/` with `sha256sum -c` → 3 of 3 OK — and
`digests-captures-2026-10-13.txt` — `README.txt` and the two captures, verified from this directory → 3 of 3
OK. `pinned-parent-bytes.txt` is verified from the repository root: 87 of 87 OK.

Disclosed rather than smoothed: **at the workspace's default parallelism this machine runs out of paging
file**, and the build — not a test — fails. The root error, and it is the only root one:

      error: could not exec the linker `link.exe`
        = note: The paging file is too small for this operation to complete. (os error 1455)

Everything else in that log follows from it: a linker killed mid-write leaves partial artefacts, and the
compiler then reports them as what they are — `error[E0786]: found invalid metadata files for crate
shploingus_clean_room`, `error: only metadata stub found for rlib dependency test please provide path to the
corresponding .rmeta file`, `error[E0463]: can't find crate for `shploingus_clean_room``, `error: crate
`voxel_mesh` required to be available in rlib format, but was not found in this form`.

Four attempts at the default `-j` were made in this pass: **one** failed with the linker error above verbatim,
**two** failed with its downstream symptoms only, and **one** passed (430 / 0, 0 error lines). Both runs
bounded to `-j 2` passed, including the one this note certifies. **Nothing in any of it is a source defect**,
the failing command is named, and the workaround is named with it. A first diagnosis written into this note
said "kache artefact churn"; the log says otherwise and the log is what is recorded.
--------------------------------------------------------------------------------
8. The structure pass — 2026-10-14, the same dirty tree, the same pin
--------------------------------------------------------------------------------

§7 landed the mesh half of the seam as one file, `clean-room/src/ported_mesh.rs` (1082 lines) holding four
concerns at once: the parent-facing adapter, the differential's accounting, the parent's horizon sizes, and
the measurement's own tests. This section is the dated cascade for the **structure pass** that split them
and gave the world its own keyed door. **Nothing above is rewritten** — §6, §7 and every number in them were
measured on their own bytes, and `digests-2026-10-13.txt` stays as that pass's dated record, which is why
its three paths no longer resolve. The pin is unchanged and re-verified: `sha256sum -c
pinned-parent-bytes.txt` → **87 of 87 OK**, over the same dirty tree; no parent byte this pass read is
outside it, and `find crates -newermt '-300 minutes' -type f` → **0**, so nothing under `crates/` was
written. No third-party crate was added.

  * `clean-room/src/ported_mesh.rs` → **`clean-room/src/port/`**, five files, one concern each:
    `mod.rs` (the directory's own account — what a port is, the P7 declaration, the pin, the four-concern
    table, and the stream half's blocker unchanged in substance), `mapping.rs` (the two matter models' one
    meeting point: the subvoxel resolution, the id bijection, the refusal translation, `Water`),
    `adapter.rs` (this world through the parent's `VoxelSource`/`ChunkNeighborhood`), `differential.rs`
    (the unit faces, the four named divergences, the parent's four horizon sizes, the comparison), and
    `tests.rs` (the measurement and its two gates). No public name was lost — the faces the tests and the
    record refer to are all still reachable at `crate::port::…` — and the module boundary now says out
    loud what §7's header said in prose: the seam and its verdict are two things.
  * **The world is the one owner of its chunk keys** (`clean-room/src/chunk.rs`):
    `CubicWorld::chunk(key)` (a key in, that chunk out or a refusal by name, which `cell` now goes
    through), `top_chunk_at(cx, cy)` and `first_chunk()` — the search §7's port used to do by probing
    cells, moved to the owner of the state it searches — and `CubicWorld::from_fields` /
    `surface_fields` / `SURFACE_FIELDS`, the one door that reads the declared surface and fills a world,
    so the client, the evidence run and the port's fixture all come through it. `chunk_band` is the band
    law the world's own span and `Footprint`'s span now share (`mesh.rs`'s span comment said they were
    "the same rule … one scale down"; now they are one function). `MESH_FIELDS` is retired into
    `chunk::SURFACE_FIELDS`.
  * **No measured number moved, and that is checked rather than argued.** The port's own measured block on
    the new bytes is **byte-identical** to the same extraction from the bytes before the pass — 249 lines,
    `cmp`-clean once the harness's own `test …` lines and trailing blanks are set aside — and it is saved
    as `port-mesh-2026-10-14.txt`. The refusal sentence §7.4 quotes reproduces byte for byte through the
    new `chunk`/`cell` composition, which is why the sentence was composed rather than rewritten.
  * **One repair the move surfaced**: the crate's own `cargo clippy --offline --all-targets -- -D
    warnings` was **red before this pass** — one lint in the port's own corner-alignment loop, in the
    bytes §7 left — and is **green now**. The loop was rewritten to the same answer (`aligned` is
    `true` exactly when every corner component is on a whole voxel, before and after) rather than
    lint-suppressed, and the port's measured block confirms the rewrite changed no count. §7's own claim
    was about `cargo build`'s warnings, which were and are 0; the clippy run is this pass's.
  * **The gates, run on the final bytes**: `cargo test --offline -j 2` in `clean-room` → exit 0, **430
    passed / 0 failed** (337 lib + 93 across the integration suites), 0 error lines, 9 733 lines of
    stdout on the final run (9 734 on the earlier full run — the figure moves by a line with the
    compiler's own progress lines, exactly as §7.6 already discloses) — the same 430, and the same three
    port tests. `cargo run --offline` → exit 0, **524** lines, the same count §41's own baseline carries.
    The `[U4]` registry row is the **only printed string this pass edited** — it now names `src/port/`
    and the date of the split — and the run reproduces byte for byte across this pass's own re-runs
    (`cmp`); a diff against §41's baseline shows every differing row as §43's own `pinned by:` moves plus
    that one row, and nothing else. The run's exit line still reads *"Clean room OK: every built system's
    acceptance bar is measured and passing."*
  * **Ordering, disclosed rather than smoothed**: the move was done as a move — the five files written,
    then the old path removed — and the old path was **untracked** (`clean-room/` is untracked in this
    repository; §40's own note names this), so §7's `ported_mesh.rs` is not recoverable byte-exact from
    this tree, only by construction from the five files plus this note. The gate the constraint asks for
    did pass, on the moved bytes: the port's measurement is byte-identical, which is the strongest form of
    *the replacement is the thing it replaced*. **Committing the clean room is still owed to the user** —
    the item §40 named — and this pass is a second instance of why.
  * **What is still owed, unchanged**: the stream half. The parent's residency still cannot be driven from
    this world — its signature is quoted in §7.4 and in `port/mod.rs`, and removing that is not this
    pass's to do. What this pass leaves is the shape that step needs: a world that owns its chunk keys
    (`CubicWorld::chunk`) and one door for its build (`from_fields`), so the **chunk provider** — one
    chunk's cells derived from the declared fields on demand, a chunk *is* a region — lands on the
    world's own keyed door rather than beside it. The horizon figures printed by the port's stream test
    are unchanged (4 913 / 15 625 / 117 649 / 7 189 057 chunks against this world's 4) and were not
    re-measured, because nothing about the world's extent moved. U10's rows 8, 11 and 13, `mc-ar-mod`,
    the `noul`/`score` sets, the U10 rulings, PATCH-040's mechanism audit, `evidence.rs`'s split, the
    per-chunk memory figure and every other named owed item are untouched; the readings' prose in its
    four homes is untouched; the flicker instrument's numbers stay the appearance seam's gate and were
    not re-measured.

This pass's own digests: `digests-2026-10-14.txt` — the twelve files it touched in the clean room
(`src/chunk.rs`, `src/mesh.rs`, `src/surface.rs`, `src/lib.rs`, the five `src/port/*.rs`, `src/systems.rs`,
`Cargo.toml` and `README.md`), verified from `clean-room/` with `sha256sum -c` → 12 of 12 OK — and
`digests-captures-2026-10-14.txt` — this file, `port-mesh-2026-10-14.txt` and the digests file, verified
from this directory → 3 of 3 OK.

--------------------------------------------------------------------------------
9. The keyed door — 2026-10-14b, the same dirty tree, the same pin
--------------------------------------------------------------------------------

§8 left the structure and named the per-chunk **provider** as owed. This section is the dated cascade for the
pass that landed it. **Nothing above is rewritten**: §7's and §8's numbers were measured on their own bytes
and stay as those passes' record (§8's paths still resolve; §7's do not, for the reason §8 gives). The pin is
re-verified on these bytes: `sha256sum -c pinned-parent-bytes.txt` → **87 of 87 OK**, and
`find crates -newermt '-300 minutes' -type f` → **0**, so nothing under `crates/` was written. No third-party
crate was added. This pass **appended** its read-set to `pinned-parent-bytes.txt` — appended, never rewrote:
the 87 digest lines that were there are untouched and still verify — and what it appended is a dated
**comment** naming the two parent paths it read with their digests, not a second digest line, because the
only parent bytes this pass read (`resident.rs` for `pub fn new(source: WorldSource)`, `world_source.rs` for
the `WorldSpec`-taking type) are **already pinned** at lines 86–87 (`e6ea0844…`, `6aa73281…`) and a pin that
states one path twice is a pin that can disagree with itself. `sha256sum -c pinned-parent-bytes.txt` →
**87 of 87 OK** (a comment is not a checksum), so the append costs the pin nothing and makes the pass's own
read-set visible in the pin rather than only in this section.

  * **What landed (production code, and it answers the question §7.4 and §8 left open).**
    `clean-room/src/chunk.rs` gains **`ChunkSource`**: the declared surface of one request, resolved **once**
    (`ChunkSource::of` from values, `ChunkSource::over` from a roster), answering either **one chunk at a key**
    (`chunk`, a whole `32³` chunk, air named where the patch has no column) or the whole patch (`world`). It is
    **one law and not two**: `column_cell` is the cell law, `named_chunks` (from `chunk_extent`) is the key law,
    `unowned_chunk_refusal` is the one refusal sentence the keyed door and `CubicWorld::chunk` both answer with,
    and the world's own doors (`from_surface`, `from_fields`) are now this source applied to every key it names.
    There is no second world model and no second derivation. The cell-level writers the old build reached
    through — `CubicWorld::set`, `CubicWorld::set_water`, `CubicWorld::chunk_mut`, `Chunk::set` — are **gone
    with the code that used them** (that is the pass's own simplification: the pour is the world's whole
    writer), and the one law they carried is now measured where it lives
    (`a_pour_is_a_set_of_the_water_it_carries_and_not_a_sum_of_pours`, replacing
    `a_second_pour_does_not_shrink_the_water_of_a_cell` — same property, through the world's real writer).
    **No public name was lost**: `from_surface` and `from_fields` keep their signatures, so every caller in the
    tree (`surface::client_world`, `evidence`, the mesher's halo build, the port's fixture) is unchanged.
  * **The world's cells did not move — measured, not argued.** The run's own output on the pre-pass bytes and
    on these bytes is **byte-identical**: 524 lines, `cmp` → 0 differing bytes, exit 0, the same closing line
    (*"Clean room OK: every built system's acceptance bar is measured and passing."*), and every digest it
    carries therefore reproduces — the U1 frame, surface-read and plane-state digests, the U12 state/chains and
    every count keyed off the cells. Both captures are in this directory
    (`run-before-the-keyed-door-2026-10-14b.txt`, `run-after-the-keyed-door-2026-10-14b.txt`), and they are
    byte-identical to each other, which is what makes the comparison auditable rather than asserted. The port's
    **own comparison block is identical in order** too: the **240** lines that are the parent-vs-this-crate
    measurement — 230 comparison lines, the **8** `mesh port differential` rows and the **2** `port-bridge`
    lines — reproduce line for line on the new bytes, each kind compared in order with `diff` → no output, and
    the `port-gate: 8 case(s) compared, 16558 unit face(s) …, 15406 divergent face(s) …, 0 unnamed` line is
    **identical to the copy §7's own capture carries** (`port-mesh-2026-10-13.txt`) — because **§8's extraction
    had dropped it**: §8's filter removed every harness line, and this gate prints on one. §8's filter also kept
    two bare `ok` lines and dropped the stream half's client line, where this pass's does the reverse. §8's file
    is 249 lines and this one 250; the **240** is the comparison itself, which is why the figure is stated per
    kind above rather than as a file length, and why this correction is dated rather than folded into the
    sentence it replaces (the commands and their output are `block-identity-2026-10-14b.txt`). Only the **stream
    half** of `port-mesh-2026-10-14b.txt` carries genuinely new measurement, and those lines are this pass's own
    two numbers (below).
  * **The differential the keyed door is measured by, and its honest limit.** The two scopes are one law, so
    "a chunk derived alone is the chunk the world holds" is *by construction* — the test that checks it
    (`a_chunk_derived_alone_is_the_chunk_the_whole_patch_holds`, over the client's 2 × 2 world and a 3 × 3 one)
    says exactly that in its own doc and is presented as a **consistency** gate, not an independent recount; an
    independent recount would need the derivation this pass replaced, and that is why the pass's own gate is
    the byte comparison above. The **real** differential with numbers is elsewhere, and there are two of them.
    (i) *Why the keyed door does not read the fields around each chunk*: the declared water law answers over the
    patch it is read on and depends on that patch's rim (`stage::Patch::is_edge` — *the rim is where the region
    is open; water leaves there*). Measured: one chunk's own 32 × 32 corner read on its own against the world's
    own 48 × 48 read of the same columns → **13 of 1024** column(s) stand a **different** water, worst
    **0.053204 m**, **8** of the 13 on the narrow read's own rim. (ii) *On the client's own subject*: the world
    this client *shows* is the declared surface **stepped** by the plane's own law (200 ticks, `SETTLE_TICKS`),
    so against the keyed door's derivation **3 673 of 131 072** cell(s) differ — the automaton's own work, named
    as such rather than as a divergence between derivations.
  * **The residency reach, re-measured, and the blocker quoted rather than described.** The parent's four
    declared horizons name **4 913 / 15 625 / 117 649 / 7 189 057** chunks around a viewer and this world owns
    **4** of them (**0.0814 % / 0.0256 % / 0.0034 % / 0.0001 %**) — those figures are **unmoved** because the
    world's extent is unmoved, and they are printed again rather than claimed. What *did* move is the kind of
    question this world answers: **one chunk key at a time** (the client's own read names 4 keys, each answerable
    alone and whole), where before the whole patch had to be materialised before any key could be answered. The
    parent's own residency **still cannot be driven from these per-chunk requests**: `resident.rs:112`
    `pub fn new(source: WorldSource) -> Refused<Self>` and `world_source.rs:107`
    `pub struct WorldSource { spec: WorldSpec, … }` — the parent's generator **by type**, with no trait, no
    generic parameter and no injection point. Both signatures are quoted with the command that read them and
    that command's output in `parent-blocker-2026-10-14b.txt`. So the mismatch now stands on the parent's side
    of the seam rather than in this world's shape, and removing it would take a parent edit (forbidden here) or
    U3's seam.
  * **Every new guard proven to bite, by perturbing its own subject** — `keyed-door-guard-bites-2026-10-14b.txt`,
    each perturbation applied to the clean room's own bytes, measured, and then restored **byte-exact**
    (`sha256sum -c` against the pre-edit copy → OK, module re-run green): **A** the provider stops refusing
    cells the patch has no column for (`patch_index(…).unwrap_or(0)`) → the edge guard fails
    (*"cell 16 of this chunk is over no column of this patch and must be air by name"*); **B**
    `stage::Patch::is_edge` stops mattering (`return false`) → the region-read measurement prints **0 of 1024**
    and its guard fails; **C** the cell law itself moves one voxel (`(z - ground)` for `(z - ground - 1)`) →
    `a_column_reads_back_the_water_the_declared_field_stands` fails **and the run's stdout differs from the
    524-line pre-pass capture** (deepest 7 809 → 7 800 µm, surface-read and plane-state digests moved, exit 1).
    C is the honest reason the record's gate is the byte comparison: no consistency gate inside this tree can
    catch a law change, because both scopes move with it.
  * **The gates, run on the final bytes.** `cargo test --offline -j 2` in `clean-room` → exit **0**, **434
    passed / 0 failed** (341 library tests — four of them this pass's own — and 93 across the integration
    suites), 18 suites, **0** error lines, 9 738 lines of stdout on the final run (the figure moves by the
    compiler's own progress lines, as §7.6 discloses). `cargo build --offline` → exit 0, **0** warnings from
    this crate. `cargo clippy --offline --all-targets -- -D warnings` → exit 0. `cargo run --offline` → exit 0,
    **524** lines, byte-identical to the pre-pass capture. This pass's own captured output:
    `port-mesh-2026-10-14b.txt` (250 lines, extracted from
    `cargo test --offline -j 2 --lib port:: -- --nocapture` by dropping the harness's own lines, as §8's did)
    and `provider-gate-2026-10-14b.txt` (the keyed door's own four measured lines).
  * **One printed row is deliberately unmoved, and it is an owe rather than an oversight.** The `[U4]` registry
    row in `src/systems.rs` still describes the port without the keyed door, exactly as §43b wrote it: this
    pass's gate **is** the run's byte identity, so editing a printed string would break the very comparison that
    proves the world's cells did not move. The row is owed to the next pass whose slice legitimately changes the
    run's stdout, and it is named here rather than left for a reader of the run to notice.
  * **What is still owed, unchanged.** The stream half of the seam is now blocked on the parent only (above).
    The field read is **not** removed and is named where it is: the keyed door derives *cells* on demand from
    a read of the world's own request. U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score` sets, the U10
    rulings, PATCH-040's mechanism audit, `evidence.rs`'s split, the per-chunk memory figure and every other
    named owed item are untouched; the readings' prose in its four homes is untouched; the flicker
    instrument's numbers stay the appearance seam's gate (§3, §6) and were not re-measured. **Committing the
    clean room is still owed to the user** — the item §40 named, and this pass is a third instance of why.

This pass's own digests: `digests-2026-10-14b.txt` — the three files it touched in the clean room
(`src/chunk.rs`, `src/port/mod.rs`, `src/port/tests.rs`) plus `README.md`, verified from `clean-room/` with
`sha256sum -c` → 4 of 4 OK — and `digests-captures-2026-10-14b.txt` — this file, the two run captures, the
port's block, the keyed door's own lines, the guard bites and the parent blocker, verified from this
directory. Two nits in this section were found **after** those digests were taken and are corrected above
rather than silently rewritten: the suite's own split (341 library + 93 integration, not 337 + 97 — `434`
was right either way) and the identity claim restated to what the commands actually verify. Both digest
files carry the correctors as a dated addendum, so the bytes as first measured stay in the record. A third
figure is disclosed rather than corrected, because it is not stable: §7.6's stdout line count is **9 738** as
this pass's own run measured it and **9 739** from a warm re-run of these same bytes — which is exactly what
§7.6's parenthesis says the figure does — while the counts the gate rests on (18 suites, 434 passed, 0
failed, 0 error lines) are identical in both runs.
--------------------------------------------------------------------------------
10. The streamed residency — 2026-10-14c, the same dirty tree, the same pin
--------------------------------------------------------------------------------

§9 left the stream half blocked on the parent and named the *consumer* as the next slice: the keyed door
answered a chunk, but nothing in the clean room ever asked it for one. This section is the dated cascade
for the pass that built the asker. **Nothing above is rewritten**: §7, §8 and §9's numbers were measured on
their own bytes and stay as those passes' record. The pin is re-verified on these bytes:
`sha256sum -c pinned-parent-bytes.txt` -> **87 of 87 OK** (with a dated read-set comment appended for the
two parent files re-read here; no digest line was added, because both are already pinned at lines 86-87),
and `find crates -newermt` -> **0**, so nothing under `crates/` was written. No third-party crate was
added.

  * **What landed (production code, and it is a consumer).** `clean-room/src/stream.rs` gains
    **`Streamer`**: a residency over one borrowed `ChunkSource`, keeping the chunks around a camera's own
    chunk while the declared `ResidencyHorizon` keeps them. Membership is **one law** — the horizon's own
    `keeps` (chunk-centre distance) — applied to the keys the **world's own source names**
    (`ChunkSource::keys`), so a resident chunk is always a chunk of this world and no key outside it is
    ever asked for. Its chunks come **only** from the keyed door: `ChunkSource::chunk` derives one chunk
    per key, on the spot. A resident chunk is **served** with no derivation; a chunk the horizon releases
    is **evicted** with its cells; a camera that comes back pays one derivation per chunk it left. The
    step is all-or-nothing (a refusal from the door changes nothing), and it is returned as a
    `StreamStep` so a caller prints what happened rather than guessing. **One owner**: membership and
    payload are one `BTreeMap` — there is no second collection to drift from it, and no second world
    model, because the only world-shaped thing the streamer knows is the source it borrows. U2's
    `world::World` keeps the *plane* world's resident **key** cache (identity, tick, anchor, plan, no
    cells); this is the cubic world's residency, and it answers with cells. The two are said apart here
    because a streaming residency that shadowed another would be the second copy SPEC-3 forbids.
  * **What a viewer can now observe, measured.** The run prints three `U4 stream:` lines
    (`stream-gate-2026-10-14c.txt`): at the declared 256 m horizon the camera's own chunk holds **4 of this
    world's 4 chunk(s)** — 131 072 cells over 4 derivations — a camera that does not move derives **0**
    more and **serves 4 of 4**, and the held cells are the world's own with **0 differing**; the **first
    eviction** is at **45 m**, because the furthest chunk this world names is **45.2548 m** from its first
    chunk (so all four are held at 46 m and three at 45 m); and a **walk** at 45 m evicts 1 chunk moving one
    chunk east and re-derives it on the way back while serving 2. Against the parent's declared four
    horizons the port's own stream half now measures the *consumer* as well as the shape
    (`port-mesh-2026-10-14c.txt`): at 256 / 384 / 768 / 3 072 m — the horizons that name **4 913 / 15 625 /
    117 649 / 7 189 057** chunk(s) around a viewer and generate every one of them — the streamer holds **4
    of this world's 4** chunk(s), 131 072 cell(s), 4 derived and 0 served. The horizon's reach beyond the
    world is not fillable here, and the numbers say so rather than a sentence: this world names four chunks
    and generates none besides.
  * **The resident set against the world it came from, and its honest limit.** The streamed chunks are the
    world's own **cell for cell**: 131 072 of 131 072 equal, 0 differing, checked per key against
    `CubicWorld::chunk`. Said plainly, as §9 said of the keyed door: the streamer and the world are **one
    law at two scopes** (`ChunkSource::derived` once per key), so that equality is a *consistency* gate
    rather than an independent recount; what is independent is the **cell-level** comparison, and the
    digest that identifies the residency (`shploingus.streamed-residency/1`, the world's own cell
    encoding) moves with its membership. And, named with numbers rather than left implicit: the residency is
    over the **declared** surface, which is the world §9 measured the *shown* one against — the client's
    drawn world has been stepped by the plane's own law and differs from this derivation in **3 673 of
    131 072** cell(s) (§9; `provider-gate-2026-10-14b.txt`), so a streaming consumer of the drawn world
    would hold the stepped cells. That is the measured reason the streamer is the clean room's own path
    rather than yet the client's. And the limit: this world is a finite patch, so a residency over
    it is bounded by the **world's own extent** and not by the horizon — the first eviction is a fact about
    the world (45.2548 m), and a camera outside the patch holds nothing at all rather than a filled horizon.
  * **The run's own movement, and it is named rather than smoothed.** Three lines and two rows' text: the
    `[U4]` registry row (§43c's owe, now naming the streamed residency and naming a ceiling on what may be
    resident at once as the remaining owe), the three new `U4 stream:` lines, and the `U4 owed after this
    seam:` line, from which the streamed residency policy comes off the owed list as landed. Stdout is
    **527** lines where §9's capture has **524**; every other line, and every digest and count the run
    prints, is byte-identical (`run-movement-2026-10-14c.txt`, the diff itself). One disclosure: the run's
    *stderr* carries one line ("serve: 7 request(s): 5 answered, 2 refused by name") printed by the `serve`
    child process, which inherits stderr; §9's capture was stdout only, so it does not appear there, and it
    is not a movement of this pass's.
  * **The port's own comparison block did not move.** The 240 lines that are the parent-vs-this-crate
    measurement are identical **in order** between §9's capture and this pass's (`diff` of each kind, no
    output), and the stream half gained exactly **5** measured lines — four per parent horizon and the
    first-eviction/walk line — for 255 lines in `port-mesh-2026-10-14c.txt` against §9's 250.
  * **Every new guard proven to bite, by perturbing its own subject** —
    `streamer-guard-bites-2026-10-14c.txt`, five bites, each applied to `src/stream.rs`, measured, and then
    restored **byte-exact** (`sha256sum -c` against a pre-bite copy -> OK after every bite, and again at
    the end): **A** the horizon stops mattering (`.filter(|_ignore| true)`) -> the first-eviction guard
    fails (*"one metre inside the far corner 45.2548 m, 4 of 4 chunk(s) are held"*); **B** a resident chunk
    is never served (`if false`) -> the still-camera guard fails (`left: 4, right: 0` derivations); **C**
    the payload comes from the wrong key -> the cells-equal guard fails (*"the payload is the chunk its key
    names"*, left `(0, 0, 2)` right `(0, 1, 2)`); **D** nothing is ever evicted (`retain(|_key, _| true)`)
    -> the walk's eviction guard fails; **E** the identity is blind to membership -> the identity guard
    fails with two equal digests.
  * **The gates, run on the final bytes.** `cargo test --offline -j 2` in `clean-room` -> exit **0**,
    **440 passed / 0 failed** (347 library tests — six of them this pass's own — and 93 across the
    integration suites), 18 suites, **0** error lines. `cargo build --offline` -> exit 0, **0** warnings
    from this crate (the five `crates/` path dependencies keep their own pre-existing warnings, which this
    pass neither introduced nor suppressed). `cargo clippy --offline --all-targets -- -D warnings` -> exit
    0. `cargo run --offline` -> exit 0, **527** stdout lines, byte-identical to the pre-pass capture apart
    from the three named hunks.
  * **The parent's residency, re-quoted rather than described.** `parent-blocker-2026-10-14c.txt` carries
    the commands and their unedited output: `resident.rs:112`
    `pub fn new(source: WorldSource) -> Refused<Self>`, `world_source.rs:107`
    `pub struct WorldSource { spec: WorldSpec, .. }` — the parent's generator **by type**, with no trait,
    no generic parameter and no injection point. So the parent's residency still cannot be driven from this
    clean room's chunks (nor from its new streamed residency) without editing a parent byte or adopting
    U3's seam. What this pass changed is the other end of the seam, not the blocker.
  * **What is still owed, unchanged and named.** A **ceiling on what may be resident at once** (a budget
    and a policy for what to drop first) is owed by name in `src/stream.rs` — the horizon is the only limit
    today. The streamer is **not** wired into the client's own reading: the client's world is one finite
    patch read whole, and its frame's numbers are that reading's evidence, so streaming it is a separate
    change whose reading movement would have to be named as one (the same rule §9 applied). The per-chunk
    memory figure, a region's own residency, `mc-ar-mod`, the `noul`/`score` sets, the U10 rulings,
    PATCH-040's mechanism audit, `evidence.rs`'s split and the parent-side blocker all stay owed and
    untouched; the readings' prose in its four homes is untouched; the flicker instrument's numbers were
    not re-measured. **Committing the clean room is still owed to the user** — §40's item, and this pass is
    a fourth instance of why.

This pass's own digests: `digests-2026-10-14c.txt` — the five files it touched in the clean room
(`src/stream.rs` new, `src/lib.rs`, `src/port/tests.rs`, `src/evidence.rs`, `src/systems.rs`) plus
`README.md`, verified from `clean-room/` with `sha256sum -c` — and `digests-captures-2026-10-14c.txt` —
this file, the pin, this pass's port block, the two run captures, the movement, the gate's lines, the guard
bites and the parent blocker, verified from this directory.
--------------------------------------------------------------------------------
11. The residency in the drawn frame — 2026-10-14d, the same dirty tree, the same pin
--------------------------------------------------------------------------------

§9 measured two derivations that disagree — the keyed door's **declared** surface against the world the
client *shows*, stepped by the plane's own law — and §10 built the consumer that streamed the declared one,
naming the gap (**3 673 of 131 072** cell(s)) as the measured reason the streamer was not yet the client's.
This section is the dated cascade for the pass that closes it. **Nothing above is rewritten**: §7–§10's
numbers were measured on their own bytes and stay as those passes' record. The pin is re-verified on these
bytes: `sha256sum -c pinned-parent-bytes.txt` -> **87 of 87 OK** (with a dated read-set comment appended:
this pass read **no** parent byte — the two paths its prose names are citations of the blocker the 10-14b/c
passes pinned at lines 86-87), and `find crates -newermt '-300 minutes' -type f` -> **0**, so nothing under
`crates/` was written. No third-party crate was added.

  * **What landed, and it is one owner rather than two doors.** `clean-room/src/stream.rs`'s `Streamer` no
    longer borrows a `ChunkSource`: it takes **the world's own store** (`CubicWorld`, the type the law steps
    and the type the drawn frame is a picture of) and holds one of its own as the residency
    (`source.bare()` gives it the frame, patch and span; each resident chunk is one key's cells through
    `CubicWorld::chunk`). Membership and payload are still one collection, and there is no second world
    model anywhere: *what cells are at this chunk* has exactly one owner — the world's own keyed door — and
    the drawn path consumes it rather than asking a derivation. `surface::{client_state, residency_of}` give
    the residency the state the client shows (`SETTLE_TICKS` of the plane's own law inside it), and
    `region_mesh` / `draw_plane` take **the residency itself**, so a frame cannot draw cells no camera holds.
    The window's own loop (`surface::present_world`) builds one residency over the plane's world and observes
    it again every tick, counting what the steps cost (`stale` / `derived` in `Presented`), and the drawn
    reading's own report now says **which cells the frame is a picture of** (`Drawing::resident`,
    `resident_cells`, `residency`).
  * **What a viewer can now observe, measured.** The run's `U4 stream` line
    (`stream-gate-2026-10-14d.txt`) prints the residency over the shown state (tick 200): **4 of this
    world's 4 chunk(s)** held — **131 072 cell(s)** over 4 derivations — a still camera deriving **0** more
    and **serving 4 of 4**, and the held cells are **the shown world's own: 0 differing of 131 072**, while
    against the **declared surface the fields make 3 673 of 131 072 cell(s) differ** — the law's own 200
    tick(s) inside the source, and §9's number, now reached from the residency's own side and asserted as
    such. The residency's identity is **the state the frame is keyed by** (`7675fd92…`, where §10's was the
    declared world's `a08101db…`), and the **first eviction** and the **walk** reproduced exactly: the
    furthest chunk is **45.2548 m** from the camera's own chunk, so the streamer holds 4 of 4 at 46 m and 3
    at 45 m, one chunk east evicts 1 and a return derives 1 while serving 2.
  * **The frame follows the residency, and the alternative is drawn beside it** (the new
    `U4 stream the frame follows the residency:` line). The residency's own 4 chunk(s) draw the drawn
    reading's own picture — **1333 rectangle(s)**, frame digest **7816bf6a…**, mesh state **7675fd92… = the
    residency's own identity** — while the *same keys* holding the **declared** surface draw where the world
    was placed: **1227 rectangle(s)**, frame digest **ac462f1c…**, **106 fewer** rectangles: the 3 673 moved
    cell(s) are 106 rectangles of the picture, which is what the two derivations were measured apart for. And
    a residency holding a **part** of the region (3 of 4 chunk(s)) is **refused by the region's own name**
    (`[parameter_out_of_range] (32, 32, 64) is not in this world: its chunk (1, 1, 2) is not one of the 3
    this world owns`), as is one holding none — so no frame is a picture of less than the region.
  * **The frame reproduced, pixel for pixel — this pass's own before/after.** The drawn reading's line in
    the run is **identical in every number it printed before this pass and after it**: quads **1333**,
    vertices 5332, triangles 2666, clipped 0, degenerate 2, **147808 of 331776** pixel(s) (44.6%), depth
    104.92..123.39 m, **47** luminance bands, channels 0x46..0xf0, frame digest **7816bf6a…**, mesh digest
    **24d61db2…**, mesh state digest **7675fd92…**, and the same camera — with exactly one clause added: the
    residency that held those cells. The **window's own** run (`window-stdout-2026-10-14d.txt`) presents the
    same picture on a real window: `U1 window: the world it drew: … the drawn frame's own digest 7816bf6a…`,
    the window's client area **is the frame's own bytes at 66 355 200 of 66 355 200 pixel(s) — every pixel,
    every frame**, and the headless drawn frame of the same state is the window's own last frame by digest
    (`7816bf6a…`, the law's own state `7675fd92…`); the frame is also written out
    (`frame-2026-10-14d.ppm`, P6 576 × 576). No window capture exists on the 10-14c bytes, so the window's
    *before* is that pass's own drawn-reading frame digest, which the window reproduces.
  * **The run's own movement, named rather than smoothed.** Stdout is **528** lines where §10's capture has
    **527**: one new line (`U4 stream the frame follows the residency:`) and **3 rows' text** — the `[U4]`
    registry row (its new clause and its `interface` line), the `U4 stream` line, and the `U4 drawn` line's
    new residency clause. Every other line, and every digest, count, pixel and camera number the run prints,
    is byte-identical (`run-movement-2026-10-14d.txt`, the diff itself). The run's stderr is the same one
    line the 10-14c pass printed (`serve: 7 request(s): 5 answered, 2 refused by name`), and `cargo run` is
    exit **0**.
  * **The port's own comparison block moved by one row's text.** `port-mesh-2026-10-14d.txt` (255 lines,
    `cargo test --offline -j 2 --lib port:: -- --nocapture --test-threads=1`, harness lines removed) is
    **identical line for line** to `port-mesh-2026-10-14c.txt` except the keyed-door line, which is re-worded
    for what is now measured: *"the declared source names 4 chunk key(s) … and is the door the camera's
    residency and the drawn frame read, so 3673 of its 131072 cell(s) differ between the two doors — the
    second path, measured, and not one the frame reads"* (§10's line said a streaming consumer *would* hold
    the stepped cells; the consumer exists and the frame reads them). Sorted, the two captures differ in
    exactly those 2 lines; in order they differ only by the test harness's own ordering.
  * **Every changed guard proven to bite, by perturbing its own subject** — `guard-bites-2026-10-14d.txt`,
    six bites, each applied to the clean room's own bytes, measured, and then restored **byte-exact**
    (`sha256sum` against a pre-bite copy -> OK after every bite, and again at the end: all five files OK):
    **d-1** the drawn door meshes the declared whole-patch world instead of the residency
    (`&client_world()?`) -> the `u4_drawn` gate fails (*"the mesh's key is the state"*) and the run's
    `the frame follows the residency` and `the frame's mesh state is the law's own state` both go **false**;
    **d-2** the law's own steps leave the shown state -> the 3 673 reproduction fails (**0** of 131 072
    differ, *"reached from the residency's own side: false"*); **d-3** the world's cell door carries another
    kind -> the part-held refusal's kind fails (`[geometry_out_of_residency]` where the guard requires
    `parameter_out_of_range`); **d-4** the held cells are compared against the declared surface -> the
    cells-equal guard fails (**3673 differing of 131072**) while the 3 673 reproduction on the same line
    stays `true` — the two guards are independent; **d-5** the cell-for-cell guard's compared world is a
    state one tick elsewhere -> it fails at the first differing cell (*"cell 10145 of chunk (0, 0, 2) is not
    the world's own"*); **d-6** the port's stream half is fed another world than the one it asserts over ->
    the gate fails (*"every one of this world's chunks must be held at each of the parent's horizons"*),
    with the printed stream lines moving to the 3 × 3 fixture's own numbers (9 of this world's 4).
  * **The gates, run on the final bytes.** `cargo test --offline -j 2` in `clean-room` -> exit **0**,
    **444 passed / 0 failed** over **18** suites, **0** error lines. `cargo clippy --offline --all-targets
    -- -D warnings` -> exit **0** (it had caught six real lints this pass had left — three unused imports,
    two `manual_ok_err` and one dead helper — and all six are fixed, not suppressed). `cargo run --offline`
    -> exit **0**, **528** stdout lines, byte-identical to the pre-pass capture apart from the four named
    hunks. A window run presents 200 frames on a real window (56.6 s, 3.53 frame(s)/s against the declared
    30) and exits **0**.
  * **The parent's residency is unchanged, and re-cited rather than re-measured.** `Resident::new(source:
    WorldSource)` still takes the parent's own generator by type (`crates/voxel-stream/src/resident.rs`,
    pinned), so the clean room's residency is still its own — the difference is now only that it is the
    residency **the drawn frame reads**. See `parent-blocker-2026-10-14c.txt` for the quoted bytes.
  * **This pass's own digests** — `digests-2026-10-14d.txt` (the nine files it touched in the clean room,
    verified from `clean-room/` with `sha256sum -c`) and `digests-captures-2026-10-14d.txt` (the captures it
    wrote under this directory, verified from here). The other named owed items are untouched: the per-chunk
    memory figure, a region's own residency, mc-ar-mod, the noul/score sets, the U10 rulings and PATCH-040's
    mechanism audit are exactly where §6–§10 left them.
--------------------------------------------------------------------------------
12. The cell owns its material — 2026-10-14f, the same dirty tree, the same pin
--------------------------------------------------------------------------------

§11 put the residency in the drawn frame and left one thing standing: the *material* a rectangle is made of
still had two owners. The cells the world is built from carry a material, and the plan reading derived the same
material a **second** time, per column, from the `lithology` class plane and the palette. Two answers to *what
is this made of* is the shape the shape ruling refused (§37, `LEDGER.md` C-189) and the one the parent's
per-column WGSL path turned into a visible material flicker — the defect this clean room exists to leave behind.
This section is the dated cascade for the pass that closes it. **Nothing above is rewritten**: §7–§11's numbers
were measured on their own bytes and stay as those passes' record. The pin is re-verified on these bytes:
`sha256sum -c docs/baselines/port-appearance-seam/pinned-parent-bytes.txt` from the repository root -> **88 of
88 OK**, exit 0, and `find crates -newermt '-400 minutes' -type f` -> **0**, so nothing under `crates/` was
written (or read) by this pass. No third-party crate was added.

  * **What landed, and it is one owner.** `src/chunk.rs`'s `CubicWorld` is the cell's material's only owner:
    `material_at(pos)` / `column(at)` is the one **read** door and `place_material(pos, material)` the one
    **write** door. `src/mesh.rs`'s sweep carries the **cell's** material into the rectangle it emits
    (`Quad::material`), so the geometry a frame is drawn from already carries the fact and no second copy of it
    exists beside the cells. `src/surface.rs`'s plan reading now takes the cells themselves —
    `Surface::render(world, cells, water, registry, light, view, zoom)` — and reads each column's material
    through them (`cells.column(at)?.material`), refusing a read that is **not** those cells' own patch
    (`parameter_out_of_range`, with both patches named) or that **disagrees about water** (the read's own water
    and the cells' own, refused with both figures) rather than shading a picture of one world's surface over
    another world's ground. The **class plane is nowhere in the appearance path**; the one material the cells
    cannot answer for is the water's, which is a palette fact and is passed in as the single declared id.
  * **What a viewer can observe, measured.** The pass's own acceptance is a **one-cell differential** on the
    drawn path (`cell-material-gate-2026-10-14f.txt`): a world built twice, the second one given **exactly one
    cell** through the world's own write door — cell `(26, 0, 75)`, material `4 -> 1` — and drawn through the
    same call the window draws through. The frame's digest **moves with that cell** (`7816bf6a…` ->
    `9cb6c6ca…`), the mesh's does (`24d61db2…` -> `13733560…`), the world's own identity does
    (`7675fd92…` -> `940ff716…`) — and the pixels drawn are **equal**, `147808 -> 147808`. A material is not a
    shape: the picture's coverage is the invariant and its colour is the variable, which is exactly the flicker's
    own subject. The same guard asserts the two worlds were **one** world before the write, and that the write
    touched one cell and no other.
  * **The product's own picture does not move — and that is the point.** `cargo run`'s stdout is
    **byte-identical** before and after this pass: 532 lines both, sha256
    `63dc1820fa48ed424821dd157f27770eff35051f7ed42b7c8b50d52bb4634b64` both, `cmp` clean
    (`run-movement-2026-10-14f.txt`). A pass whose subject is *who owns a fact* rather than *what is drawn*
    owes exactly this: the same bytes out of the same product, with the fact's owner moved. The new number this
    pass owes is the one-cell differential above, and it is a **test** rather than a printed line — so no run
    line was added, and none was needed.
  * **The window, on these bytes.** A real window presents the same picture: `window-stdout-2026-10-14f.txt`,
    200 of the declared 200 frame(s) in 51.4 s — **3.89 frame(s)/s measured** against the declared 30 — the
    window's own client area the frame's own bytes at **66 355 200 of 66 355 200** pixel(s), frame digest
    `7816bf6a…` (the headless drawn frame of the same state by digest), 47 luminance band(s) on 1333 shaded
    rectangle(s), materials `0->1 1->1 2->2 3->3 4->4 5->6` with water = material 5, and the screenshot written
    as `frame-2026-10-14f.ppm` (P6, 576 × 576). The window's own numbers reproduce the capture taken earlier in
    this pass line for line, the measured rate aside (a timing, not a derived number); against §11's shipped
    window capture the same digests and counts stand, and the bars that differ there are 10-14e's own (the
    person's keys and the frustum bar), not this pass's.
  * **Every changed guard proven to bite, by perturbing its own subject** — `guard-bites-2026-10-14f.txt`, six
    bites, each applied to the clean room's own bytes, measured, and then restored **byte-exact** (`sha256sum`
    against a pre-bite copy -> OK after every bite, and again at the end: all six files OK): **f-1** the
    material **write** door stops writing -> the differential cannot even be set up (*"left: Some(MaterialId(4))"
    where the guard requires `Some(MaterialId(1))`*); **f-2** the **sweep** stops reading the cell's material ->
    the frame digest does not move, and against the tightened guard the **rectangle set itself** fails to move
    (*"the geometry follows the cell: the face that changed is a different rectangle set"* — re-run on the final
    test bytes, and this pass's own note records both, because the earlier run was against the pre-tightening
    test file); **f-3** the plan reading stops taking the column's material from its cell -> *"one cell's material
    is the frame's own: the picture follows the cell"* fails on equal digests; **f-4** the read/cells **patch**
    guard is disabled -> the refusal the guard demands by name is gone and another one answers in its place
    (`parameter_out_of_range: (0, 0) is outside this world's own patch: it spans x 1..48, y 0..47`); **f-5** the
    one conversion stops refusing a material with no reflectance -> `unwrap_err()` on `Ok(Surface {
    reflectance: None, … })`; **f-6** the read/cells **water agreement** guard is disabled -> the frame is drawn
    where it must be refused. Six files, six subjects, no guard passed by accident.
  * **The gates, run on the final bytes.** `cargo test --offline -j 2` in `clean-room` -> exit **0**,
    **458 passed / 0 failed** over **18** suites, **0** error lines. `cargo clippy --offline --all-targets
    -- -D warnings` -> exit **0** (the path-dependency crates' own `missing_docs` warnings are unchanged and
    are not this crate's lints). `cargo run --offline` -> exit **0**, **532** stdout lines, byte-identical to
    the pre-pass capture. A window run presents 200 frames on a real window and exits **0**.
  * **What is still owed, unchanged and named.** The **GPU pass** (T009's other half — a shader checked against
    `presentation::shade`); a **region's own residency** so the drawn mesh can be wider than one chunk, with
    water's own side faces; the **per-chunk memory figure**; the **displaced (256×) surface**; exit (b) — the
    surface's remainder inside the voxel; `evidence.rs`'s split and `src/mesh.rs`'s module split; a resizable
    window and the Sun's azimuth; a ceiling on what may be resident at once; the parent's **non-portable
    residency** (`Resident::new(source: WorldSource)`, still re-quoted in `parent-blocker-2026-10-14c.txt` and
    `-2026-10-14e.txt` rather than re-measured); U10's rows 8, 11 and 13, `mc-ar-mod`, the `noul`/`score`
    calibration sets, the score instrument's rungs and U2's axis vocabulary. The readings' prose in its four
    homes is untouched. **Committing the clean room is still owed to the user** — §40's item, and §11's, and
    this pass is a fif
    this pass is a fifth instance of why.
  * **A record gap this pass did not fill but does name.** The 10-14e pass (*the person drives the client*:
    `viewpoint.rs`, `controller.rs`, `surface::step_driven`, the binary's `drive` mode) wrote its captures under
    this directory and its own registry rows, but left **no numbered entry** in `clean-room/README.md`'s
    history block — §43e's entry is the last one before this pass's §43f. This pass did not invent one for it;
    its record is its captures (`run-after-the-person-drives-the-client-2026-10-14e.txt`,
    `guard-bites-2026-10-14e.txt`, `frame-2026-10-14e.ppm`) and the `[U1]` row they print.
  * **This pass's own digests** — `digests-2026-10-14f.txt` (the seven files it touched in the clean room,
    verified from `clean-room/` with `sha256sum -c`) and `digests-captures-2026-10-14f.txt` (the captures it
    wrote under this directory, verified from here).
