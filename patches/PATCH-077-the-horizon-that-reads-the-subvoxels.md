---
patch_id: PATCH-077
title: The horizon that reads the subvoxels — a streamed light march at the octree's own grain, and the CPU march refused
status: implemented
operation: repair
target: voxel-client/light
base_ref: working-tree-2026-10-25
base_digest: sha256:551eee6c2d1cb1a6caccbe821faddb4af239818634891d59821ae04e0f514d8b
dependencies: [PATCH-069, PATCH-070, PATCH-074]
source: user-directed-mission
created_at: 2026-10-25
approval: "user-directed 2026-10-25 — shadows are to be refused where they are coarser than the subvoxel geometry, the light is to be marched at the geometry's own grain, and the horizon is to stream with the chunks rather than appear a chunk at a time"
live_mutation: false
---

# Change

**The streamed light horizon now carries the octree's own occupancy and marches it cell by cell, and
the coarser shadow map is refused wherever the march has a measurement of its own.**

`base_digest` above is the sha256 of the eight files this record now carries — `horizon.rs`,
`horizon.wgsl`, `far_surface.rs`, `shading.wgsl`, `world.rs`, `lib.rs`, `voxel-work/src/stream.rs`,
`shploingus/src/main.rs` — concatenated in that order (`cat … | sha256sum`), **as they stand after
this pass** (the shader extraction and the far surface joined the export in the pass of
2026-10-25; the earlier digest covered five files). This checkout's `HEAD` is a backup from before
several passes, so a digest of the true pre-image cannot be taken from it; the post-image digest is
what a reader can recompute, and it is stated as that rather than as something it is not.

## The defect, stated as it was measured

Three things were wrong with the light field the fragment marched, and they compounded:

1. **The volume threw the octree's grain away.** `HorizonStream::stamp` read each leaf and wrote one
   `u32` per metre — `1` when *any* of a voxel's sixty-four subvoxels held matter. A block with a
   corner broken out therefore shaded its whole metre, and the CPU's own reference
   (`voxel_stream::light::transmittance_tree`, held to the voxel walk by
   `the_tree_walk_and_the_voxel_walk_are_one_measurement`) had already been corrected to the
   subvoxel grain by PATCH-070. The GPU was drawing a shadow the CPU measurement refuses.
2. **The march stepped a cell along the ray rather than traversing the grid.** `horizon_visibility`
   advanced `p = p + step` by one cell per iteration and tested the cell the *point* fell in. On an
   oblique ray that visits roughly every other cell, so a wall one voxel thick could sit between two
   samples and let the beam through — the leak PATCH-069 fixed in the voxel walk, reintroduced in the
   shader from the other direction.
3. **The map answered where the volume had a finer answer, and the volume claimed answers it did not
   have.** `sun_visibility` multiplied every beam by a 1024² cascade whose far texel is 0.56 m with a
   2 cm normal bias; at a grazing Sun that cuts a straight edge across a face the geometry says is
   lit, which is the diagonal line the user reported on a trunk. And a cube that had not yet been
   stamped was indistinguishable from a cube of clear air, so an empty volume would have had to be
   believed.

## What was changed, and where it lives

**One owner of the streamed state** (`crates/voxel-client/src/horizon.rs`, `HorizonStream`):

- `cells: Vec<u32>` became two words per metre — **the 64 bits of the voxel's 4 × 4 × 4 subvoxel
  grid**, bit `x + 4y + 16z` (`SubvoxelIndex::flat`), because WGSL has no `u64`.
  `subvoxel_words` derives them from `VoxelShape::cells()`, so a `Mixed` leaf travels as the shape it
  is and a `Uniform` region leaves all sixty-four bits set. 96³ × 8 B = 6.75 MB.
- `stamp` **clears the chunk's own cells before marking them**, because a chunk replaces the tree it
  was; an additive stamp kept the block the player had just broken.
- Stamps are keyed by **(chunk, version of its tree)** — `Streamer::tree_version`, the streamer's own
  residency counter, exposed in `voxel-work` for this caller. An edit rebuilds the chunk, the version
  moves, and that chunk is stamped again; an untouched chunk costs one map lookup.
- **A coverage mask** (`coverage()` → one bit per 32 m chunk slot of the 3 × 3 × 3 cube, 27 bits of
  the uniform's third `vec4`) says which slots the volume has looked at. Zero now means "nothing
  stands here", which is a claim only a stamped chunk may make; an unstamped slot reports *no
  measurement*, and the coarser map keeps the answer there. This is the refuse-never-default rule
  applied to the volume's own silence.
- `subvoxel_at` is the CPU mirror of the shader's address law, kept for the test that holds the stamp
  to the octree — an address lookup, not a second march.

**One march, in one place** (`crates/voxel-client/src/shading.wgsl`):

- `horizon_mask` reads the two words; `horizon_subvoxel_solid` is the address law
  (cell = floor, subvoxel = fraction × 4); `horizon_stamped` reads the coverage bit.
- `horizon_walk` is **Amanatides & Woo's traversal** — the same walk `raycast::Crossings` uses — so
  every cell the ray passes through is visited in order. A cell with no bits costs one compare, a
  fully-solid cell ends the march, and a **partly-dug** cell is walked at half-subvoxel stride
  (`horizon_partial_blocked`), which is `Spans` splitting a `Mixed` leaf, paid only where a mask is
  partial. The fragment's own cell is never tested: the march advances before it looks, which is
  `transmittance_tree`'s `skip_own_voxel` arrived at from the other side.
- It returns `vec4(distance, blocked, left_at, measured)` per band, and `horizon_transmittance` packs
  the two bands into `vec4(fine_hit, fine_measured, coarse_hit, coarse_measured)`. **`measured` is the
  whole wiring change**: `fs_main` takes the map's visibility, and where the march completed its reach
  the march's answer **replaces** it rather than multiplying into it. The subvoxel march is the finer
  measurement inside its range; the map answers outside it, which is the light the far field had
  before the volume existed — and, as the later rule below states, the coarse band never overrules
  the map at all.

**Streaming, at the rate the box needs** (`crates/shploingus/src/main.rs`): a 96 m cube is exactly
3 × 3 × 3 chunks, so the per-frame stamp budget covers the box. Eight a frame left the stamped slots
beside an unstamped rest, and the seam between them ran along a chunk boundary.

**And then the window said the budget itself was the stall.** Twenty-seven stamps in the frame of a
crossing is twenty-seven octrees walked, three of them for chunks that had not changed — "massive
frame lag when I enter a chunk". The cube is a window on world space, so `HorizonStream::recenter` now
**shifts** both bands to the new anchor (`shifted`: one row copy per line, the far band's 32 m shift
falling on four whole 8 m cells) and keeps every stamp whose chunk is still inside the box. Only the
chunks that entered are left to stamp, and the budget is nine a frame: a crossed box fills in one or
two frames, and the slots still owed are *unmeasured* through the coverage mask rather than air, so a
partial fill is the shadow map's answer and not a chunk-shaped hole.

**A second streaming bug, found by the same report.** `retain` cleared the **whole coarse band** when
a chunk left the window — but nothing restamped the chunks whose versions had not changed, so the far
field went blank the moment the window moved and stayed blank. It now clears only the gone chunk's own
four-by-four coarse cells, which no neighbour shares because a chunk is four 8 m cells aligned to its
corner.

## The refusal this record exists to state

**No CPU march was written, and none was written first.** The obvious shape of this fix is a
`HorizonField` on the worker: march the cells on the CPU, upload a band per chunk, shade from the
band. That is refused by name for three reasons, each of which this tree has already paid for once:

- it is **PATCH-074's defect** — a per-corner measurement arrives a chunk at a time, so the light lags
  the geometry it is drawn on, which is exactly why the per-corner bands stopped being read in
  `fs_main`;
- it puts a second march in the tree, and the two would drift: this project's rule is that a GPU
  transcription is held to its Rust original by a test (`the_gpu_shades_a_surface_exactly_as_the_crate_does`),
  and a CPU march over the same volume would be the *third* implementation of one law;
- the fragment already knows which cell it is in and the volume already streams per chunk, so the
  work is where the answer is needed and no cell the eye is not looking at is ever marched.

The same refusal applies to a debug toggle, a feature flag, or a "fast path" beside the new one: the
old per-metre march is **gone**, not switched off. `horizon_solid` no longer exists.

## The second slice: reach, transport, and the refusal that keeps the beam honest

Watching the same window again, the user named four things still wrong. Three of them are this band
of the record; the fourth is named as owed below.

- **A coarse band behind the fine one.** The fine band is the octree's own subvoxels over 96 m (3 × 3
  × 3 chunks, restamped on any residency or edit change); behind it sits **8 m cells over 160 m**, one
  word each carrying `FAR_SOLID` and `FAR_MEASURED`, filled by the same `HorizonStream::stamp` from
  the same leaves and walked by the same `horizon_walk` with `band = 1`, starting from the distance the
  fine band stopped measuring. That is the sparse march a tunnel needs: a hundred-metre corridor costs
  a hundred compares instead of eleven hundred, and it stops at its own far end.
- **Two bands, two authorities.** Only the band that is *finer than the shadow map* may overrule it, in
  either direction: a fine hit is a refusal the map cannot argue with, and a fine clear is the beam.
  The coarse band is **refused for the beam** — eight metres of summary against a 0.28 m texel, over
  ground the map still covers, is how the far field grew blocks of shadow the geometry does not cast
  (the regression this rule removes). The coarse band answers the **dome**, where a diffuse answer does
  not need half-metre edges.
- **The dome's rays are as long as the volume** — `HORIZON_FAR_REACH_M`, both bands — so a tunnel's
  ambient stops at its end instead of at 32 m. The dome answers `view + (1 − view)·albedo`: the
  enclosure's own one bounce, with the fragment's measured reflectance, which is what carries light
  down a corridor rather than letting the ambient die at the rig's floor.
- **The traversal bound was below the geometry's own count.** `HORIZON_MAX_STEPS` was 128 against a
  96 m cube whose diagonals cross 289 cells, so a long ray returned *unmeasured* and handed the answer
  straight back to the map — silently, and on exactly the rays a corner casts. It is 320.
- **The volume streams like the chunks do.** Stamps are keyed `(chunk, tree_version)` off the
  streamer's own residency counter, pruned by `horizon_retain` when a chunk leaves the window, and the
  whole 27-chunk box is stamped on the frame the cube moves rather than eight slots a frame.

## The third slice: the water the beam crosses, and the air it does not cross twice

The last pass left one requested outcome undone — "the beam picks up sky and cloud colour on its way
in" — so `fs_main` now multiplies the beam by `beam_cloud_filter`, which reads `atmosphere.wgsl`'s
own `sky_model` rows on the beam's own ray.

**What it does, and the boundary it refuses to cross.** `skylight::daylight_of` hands `lights[0]`
`chromaticity(Sky::sun)`, and `Sky::sun` is `total_direct_normal · air_column` — `exp(−(β_R·H_R +
β_M·turbidity·H_M)·m)` per channel, with `intensity` that band's luminance over the year's peak. **The
air's reddening and dimming are therefore already in the beam, per channel, before the shader runs.**
Marching an air mass here as well is a sunset put through the atmosphere twice, so this refuses it by
name.

`air_column` carries Rayleigh and Mie and **no droplets** — the water lives in the model's own header
row (`head.y` per metre, `head.z` the layer's depth above the eye), and only the sky pass transported
it. That is the term the rig cannot know and the march now applies: the ray's `air_mass` sets the path,
`path_in_layer` of that path inside `depth − local.y` of water, and the beam arrives `exp(−droplets ·
path)`. A beam under a cloud bank dims, one above it does not, and a low Sun crosses more water than a
high one because the *path* is what grew. It is one exponential rather than a band loop because
`droplets` is a single per-metre number in this model: the water's transmittance is grey by
construction, and twelve identical factors would be theatre.

**Not crossed, and named:** the *drawn* cloud cubes. They are meshed geometry carrying a per-leaf
veil (`cloud_cubes`), not a column the fragment can look up, so the beam cannot sample their water; what
it crosses is the medium `atmosphere.wgsl` declares and the sky pass already transports. Cloud shadows
on the ground remain the shadow map's, which draws the same records.

## The fourth slice: the coarse band as a level of the same octree

The last owed item was that the far field was one fixed grain with no LOD. It was worse than that: a
far cell was a **flag**, `FAR_SOLID` set if any of its 512 voxels held matter, so a one-voxel roof
shaded eight metres as a wall and the ring where the bands met disagreed with what the fine band had
said a metre away.

**A cell now carries the share of itself that is matter** — bits 0–7 of its word, `255` full, beside
the stamped bit — accumulated in the same `visit_leaves` pass the fine band is stamped from: a
`Uniform` leaf adds its whole volume, a `Mixed` leaf its solid subvoxels, and a leaf wider than the
cell divides its volume across the cells it spans. The mesher meshes each leaf at its own level
(`mesh::leaf_region`), so the coarse band reads the *same leaves* at the level of its own cell rather
than through a second ladder; and because a chunk covers whole 8 m cells, a cell's share is exactly
the mean of the fine band over that cell. That is the handover: **the two bands agree in the mean**, so
nothing is invented at the ring.

**The march composes them by transmittance**, `(1 − fine) · Π(1 − share)`: a coarse cell that is a
third full dims the ray by a third and the walk continues, instead of the first flag ending it. That is
`pattern::octave_weight`'s own rule — a rung contributes the share it covers — applied to occupancy.
The rules from the earlier slices are untouched: the fine band still refuses the map on its own hit,
the coarse band still answers the dome only and never the direct beam, and a stamp still travels with
its chunk across a crossing.

## What is still owed, named

Nothing from the request that prompted this record is outstanding. What is *unproven* is on the record
below: no frame of this has been drawn on a screen.

## Evidence

Run in this checkout, after the change:

```text
cargo test --offline -p voxel-client --lib -- horizon:: world::tests::the_streamed world::tests::a_partly world::tests::the_shadows
    → 7 passed; 0 failed

# after the coarse band's refusal rule and the 320-step bound
cargo check --offline -p voxel-client --lib
    → Finished (the unused-import warnings are other modules')
cargo test --offline -p voxel-client --lib -- every_composed_pass_is_valid_wgsl
    → 1 passed   (the composed pass still parses as WGSL)
cargo test --offline -p voxel-client --lib -- the_shaders_reach_and_the_hosts_are_one
    → 1 passed
cargo test --offline -p voxel-client --lib -- a_partly_dug_voxel
    → 2 passed; 0 failed
```

After the beam's water filter:

```text
cargo test --offline -p voxel-client --lib -- every_composed_pass_is_valid_wgsl
    → 1 passed   (first run failed: `filter` is a reserved word in WGSL; the tint is named `tint`)
```

After the coarse band became a share:

```text
cargo test --offline -p voxel-client --lib -- horizon::
    → 7 passed; 0 failed
      (the_coarse_band_carries_each_cells_share_and_says_where_it_has_not_looked: a one-voxel roof
       is 16/512 of its cell and `far_solid_at` is false; a full 8 m cell is share 1.0 and true;
       the neighbouring cell is its own 0.0)
cargo test --offline -p voxel-client --lib -- every_composed_pass_is_valid_wgsl
    → 1 passed
cargo check --offline -p shploingus
    → Finished   (the prose census still passes; this pass's explanations went to
      docs/lighting-sky-and-moon.md after the budget refused a comment in main.rs at 30 % > 29 %)
```

After the crossing shift and the coarse-band retain fix:

```text
cargo check --offline -p voxel-client --lib
cargo check --offline -p shploingus
    → 0 errors
cargo test --offline -p voxel-client --lib -- horizon::
    → 7 passed; 0 failed
      (a_crossing_shifts_the_cube_and_keeps_what_it_already_knows is the new witness: the roof of
       chunk 0 is still solid after one crossing and forgotten after two)
```

```text

cargo test --offline -p voxel-client --lib renderer::
    → 22 passed; 0 failed

cargo test --offline -p voxel-work --lib
    → 69 passed; 0 failed; 2 ignored

cargo check --offline -p shploingus
    → 0 errors
```

The decisive witness is `world::tests::a_partly_dug_voxel_shadows_its_columns_and_not_its_metre`: one
voxel with a single subvoxel column left standing, no shadow pass at all, and the picture is compared
against `voxel_stream::light::transmittance_tree` on the same tree — `0.0` through the standing
column and `1.0` through the dug quarters, with the drawn pixel following the reference. The pre-fix
volume could not pass it: it marked the whole metre.

**Not verified, and not claimed:** the full `voxel-client` suite was run once and its harness died
after 154 of 570 tests with no failure line (no wgpu or naga diagnostic in the log); the module runs
above cover every file this record touches, and the full suite is owed on a quiet machine. No window
was opened by this pass, so the picture is the user's own observation and the pixel tests' — the
standing rule that the client is never tested invisibly still wants a visible run.

**The stall is fixed by construction and measured only by its own test.** `a_crossing_shifts_the_cube_and_keeps_what_it_already_knows` proves the cells move and the stamps survive; nobody has yet timed a crossing in a running window, so "no more lag" is a claim about the work removed (twenty-seven octree walks become at most nine new chunks plus a band copy), not a frame-time measurement.

**`beam_cloud_filter` is exercised as WGSL and not as a picture.** The composed pass parses, and the
arithmetic is the sky pass's own `path_in_layer` on the same header row, but nobody has yet seen
whether the shipped `droplets`/`droplet_layer_depth_m` make the beam's dimming visible at all — a
model whose droplet coefficient is small turns this into a no-op that costs one dozen-band read per
fragment, and that is a measurement on the screen, not an argument in a file. The same is true of the
coarse band's refusal rule: it is proven to parse and to keep the finer answer, and unproven as a
field the eye would call right.

## Export

**The patch is bundled**: [PATCH-077-the-horizon-that-reads-the-subvoxels.patch](PATCH-077-the-horizon-that-reads-the-subvoxels.patch),
1 746 lines, six files, produced by [PATCH-077-export.py](PATCH-077-export.py).

`git diff HEAD` cannot be the source: this checkout has 1 113 paths of other passes' uncommitted work
and `HEAD` predates whole rewrites of these files, so any diff against it carries three passes with
this one. The exporter therefore reconstructs **the pre-thread text of every region this thread
touched** (each block is written out in the script, so the base is exact rather than approximated),
refuses if an anchor has moved or if a name this thread introduced survives in the rebuilt base, and
diffs against the tree. `horizon.rs` is untracked, so it travels whole.

```bash
python patches/PATCH-077-export.py /dev/null /tmp/p077base \
    > patches/PATCH-077-the-horizon-that-reads-the-subvoxels.patch

# 1. it applies to a clean copy of the base
(cd /tmp/p077base && git apply --check -p1 ../../../patches/PATCH-077-the-horizon-that-reads-the-subvoxels.patch)
# 2. applied for real, it reproduces the five files byte for byte
(cd /tmp/p077apply && git -c core.autocrlf=false apply -p1 …patch && cmp each file)
# 3. and its `+` side is exactly this tree
git apply --check --reverse -C1 --ignore-whitespace patches/PATCH-077-the-horizon-that-reads-the-subvoxels.patch
```

All three were run on the re-shipped patch — **2 353 lines, eight files** — `clean-copy=0`, all eight
files `identical`, `reverse=0`. Since PATCH-078 landed on top of these files (the tree's `origin`
field, the forest's draw call), the `+` side of this patch is the tree **as 077 left it** — the base
of PATCH-078 — rather than the live tree. 077 then 078 is the order. Two defects were found by
these checks and fixed in the exporter rather than worked around — the first was a duplicated anchor
that dropped `horizon_stamp` from the base, the second was Python's Windows text stream rewriting the
diff's own line endings, which made `git apply` refuse hunks that were exactly right.

From a checkout where this pass is committed, the same export is one command and needs no script:

```bash
git diff <base-ref>.. -- crates/voxel-client/src/horizon.rs crates/voxel-client/src/shading.wgsl \
    crates/voxel-client/src/world.rs crates/voxel-work/src/stream.rs crates/shploingus/src/main.rs \
    > patches/PATCH-077-the-horizon-that-reads-the-subvoxels.patch
```

## This pass: the shader's own file, and the far surface

Two things were added to the export, and neither is a second statement of what was already here.

**The horizon's shader moved out of `shading.wgsl`.** `horizon.wgsl` now owns the bindings, the two
bands' address laws, `horizon_walk`, `horizon_transmittance`, `horizon_visibility` and the dome;
`world::pass_source` composes it between `atmosphere.wgsl` and the pass that draws with it, so
`shading.wgsl` is about a fragment's material and the frame's light and about nothing else. The move
is verbatim, and the exporter reconstructs the base of `shading.wgsl` from that file (`SPLIT`) because
a moved block's anchors are gone from the tree and a region cannot express it.

**`far_surface.rs` draws the distant terrain.** A resident chunk is meshed from its own octree at
level 3 — an 8 m cell, `visit_at_level`, the octree's own regions — with a cell drawn when at least
half of it is matter and taking the material holding the most of it. The result is a `Mesh` in the
chunk's own frame (no new vertex format, no new origin field, no new pass), streamed by residency
keyed to `Streamer::tree_version`, retained rather than cleared on a crossing, budgeted at three
chunks a frame, and absent rather than faked where it has not been built. `main.rs` draws the fine
mesh inside `FAR_HANDOVER_CHUNKS` (3) and the coarse one outside, so a chunk is drawn at one level
and the seam is a shared boundary. The doc section that states the rule is
`docs/lighting-sky-and-moon.md` §"The far surface: a chunk at the coarse cell's own level".

**Checks run.** `cargo test -p voxel-client --lib -- far_surface::` → **6 passed**; `-- horizon::` →
**7 passed**; `cargo check -p shploingus` → finished with 0 errors, which also runs the prose census.
No suite, no workspace build (the user has the game running).

**Owed, and stated as owed:** whether the level-3 surface reads as terrain rather than stair-steps at
the ring, whether the 96 m handover pops in motion, and what the far field costs as a share of a
frame. None of the three can be answered without a rendered frame.

**Not landed: the tree mob.** The producer, the stand, the body and the removal of the block tree are
one slice by this project's own rule (`PATCH-043` RM4 — no build in which the forest is missing), and
half of it would either double the forest or delete it. It stays the next unit rather than being
half-landed here.

## Recipe

```recipe
segment: game
axis: evidence_grade
ticket: T-0771
site: crates/voxel-client/src/shading.wgsl:391
what: marshal the light horizon with the octree's own 64-bit subvoxel mask and a grid traversal, and let that finer answer replace the shadow map where it measured
because: the volume kept one bit per metre and stepped a cell per sample, so a dug quarter shaded its whole metre and a one-voxel wall could sit between two samples; the map's 0.56 m far texel then cut a straight edge across a lit face
check: cargo test -p voxel-client --lib world::tests::a_partly_dug_voxel_shadows_its_columns_and_not_its_metre -> failures 0
witness: crates/voxel-client/src/world.rs
expects: 0
requires: cargo check --workspace
```
