---
patch_id: PATCH-084
title: The far field at eight metres, lit by the band that already measured it
status: applied
operation: feature
target: voxel-client/far-terrain
base_ref: working tree at 2026-10-26, after PATCH-083
dependencies: [PATCH-080, PATCH-081, PATCH-083]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'build the smallest honest version as one vertical slice: one coarse mesh per far chunk derived from its octree at the 8 m rung, drawn beyond the residency window ... lit by the coarse band and shadow sampled by the far path that already exists, with the cell size carried so the material ladder picks its rung'"
live_mutation: false
---

# What was missing

The only terrain drawn was the streamed window. A chunk that left the window left no geometry
behind, so past 160 m the horizon was sky regardless of what the world did there — and the coarse
band, which already answers 8 m cells for light, had nothing at its own rung to light. The user
asked for low-density cubic terrain twice; this is the first vertical slice of it.

# The slice, end to end

1. **One census** (`far_terrain::census`) walks a chunk's subvoxel octree and measures each 8 m
   cell: solid voxels out of its 512, and the material at the top of its column. The coarse light
   band's own summary now calls this function, so **the face a fragment sees and the share the
   march reads are one measurement**.
2. **One coarse mesh** (`far_terrain::coarse_mesh`) emits the mesher's own `Mesh` and `Quad` types
   on the coarse lattice: slice, `uv` and extent are multiples of 32 subvoxels, and a greedy merge
   joins equal-material cells exactly as the subvoxel mesher joins equal-material faces. A cell is
   drawn solid at a share of one half — the band's own `far_solid_at` threshold.
3. **A ring on its own thread** (`FarTerrain`) begins one chunk beyond the streamed window and is
   two chunks deep, one vertical layer at the eye's chunk. It censuses one layer past what it
   draws, so every face between two far chunks knows both sides; a face into an uncensused
   neighbour is **not drawn**, so the slice's outer edge ends in air rather than in a wall of
   eight-metre stones. One job at a time; a crossing during a job is remembered and scheduled
   against the eye's newest chunk rather than queued behind a stale ring.
4. **Its own record pair in the scene and the pass** (`Scene::far`, `Scene::far_origins`,
   `WorldPipeline`'s far buffers). The pair is separate for the cloud's reason: a far chunk is
   built off the frame and must not be renumbered by a chunk landing near the eye. The records are
   drawn by the same `vs_quad` and shaded by the same `fs_main`.
5. **The cell size rides the record's spare slot** (`light_y`, zero for every rectangle before
   this), and `vs_quad` hands it to the fragment as `cell_m`. `fs_main` floors the pattern's
   footprint with it, so an eight-metre face reads the eight-metre rung instead of a half-metre
   detail its cells do not hold. Fine rectangles carry `0` and are unchanged.
6. **The band is stamped before the frame draws it**: `HorizonStream::stamp_far` writes the same
   census's shares into the coarse band, and the app does this on the frame it adopts a chunk,
   before the scene is taken. No far face is drawn before its light is measured.

The fine path's light law is untouched: the fine stamp still marks its subvoxel cells as before,
`stamp` only gains the shared census for its coarse words, and the two new horizon tests pin the
census stamp to the tree stamp cell for cell.

# Evidence

**The mesh is at the rung.** `a_far_chunk_is_meshed_at_the_eight_metre_cell_and_has_surface`:
every drawn quad's plane, `uv` and extent is a multiple of 32 subvoxels, every extent at least one
cell, and the mesh's origin is the chunk's corner.

**The faces are lit, every one of them.** `a_far_chunks_surface_is_lit_by_the_coarse_band_it_stamped`:
a real chunk (4, 0, 4), its census stamped into a `HorizonStream`, and for **every drawn face** —
10 of them — the cell half a metre out along its normal answers `Some`, and the cell half a metre
**in** answers a nonzero share. So the lit fraction is every face (10/10), not a subset: no face is
lit by no measurement, and none is geometry the band calls air. One either way fails the test.

**The census is the band's.** `the_census_a_far_mesh_uses_writes_the_same_band_words_as_its_tree`:
two streams, one stamped from the tree and one from the census, agree on every one of the chunk's
64 coarse cells.

**The census accounts for the tree.** `the_census_sums_to_the_chunks_own_voxel_count`: the cells'
solid volumes sum to `tree.solid_count()` exactly.

**The record path draws and states its rung.** `a_far_mesh_is_drawn_and_states_the_cell_it_was_cut_at`:
a synthetic 16 m block, `Scene::with_far`, `Renderer::render` — the centre pixel is covered
(alpha 1.0) and not black, and the record's spare slot decodes to `8.0` m.

**The app draws it.** With `SHPLOINGUS_PROBE_DEBUG=1`, the running binary reports
`far: 104 of 104 chunks, 386 quads` — the same ring and quad count the harness builds. The debug
line is printed from the frame loop, so the chunks have been adopted and appended to the scene that
frame drew.

# Cost

`frame-cost`, same scene, camera and machine, with and without the far field. Two clean runs,
same build; the dossier's rows are the earlier run this sitting:

| frame | 1280x720 | 640x360 | ratio |
|---|---|---|---|
| the window alone | 21.08 / 21.11 ms | 7.67 / 7.75 ms | 2.75 / 2.73 |
| window + far field (104 chunks, 386 quads) | 21.18 / 20.82 ms | 7.73 / 7.58 ms | 2.74 / 2.75 |

**The far field's price is +0.10 and −0.29 ms in the two runs, both inside the ±0.6 ms run-to-run
band**: 386 rectangles at eight metres are below this harness's noise floor. The honest statement
is not "free" but "too small for this instrument"; the geometry is 386 quads against the window's
57 175. The window-only median sits 0.06 ms from PATCH-082's 21.02 ms, so this sitting's baseline
is the same machine the previous pass measured.

**Its generation is not free and is not on the frame.** The ring censuses 168 chunks and meshes
104, and took **15.6 s of one background thread** in the harness (`far field: 104 of 104 chunks,
386 quads, generated in 15 643–15 677 ms (one thread)`); in the running app, competing with the
streamer's own first fill, the same ring had fully arrived at about the 38 s mark of a 40 s run. A
standing eye pays this once; a crossing asks for the leading arc (about 33 censuses, ~3 s of
background work), and the ring draws as it arrives.

**One contamination, recorded because it would otherwise be blamed on this change.** The first
re-measurement after the ring landed read **34.9 ms** in both the full and the quarter frame, and
a quarter ratio of 2.1. A `shploingus` instance started earlier in the session was still drawing
from the same GPU; stopping it restored 21.08 ms and a 2.75 ratio on the same binary, twice. The
number to compare against is therefore the clean pair above, not the first reading.

# Limits, named

* **One vertical layer at the eye's chunk.** A far peak taller than 32 m above the eye's chunk
  floor is cut at the top of that chunk.
* **One far rung.** There is no ring beyond `radius+2` and no 64 m terrain; rungs above this one
  remain the work `docs/lod-ladder.md` lists.
* **The half-volume threshold quantises the skyline.** A hillside whose surface crosses a cell in
  its lower half loses that cell, so far heights are up to 8 m coarse by construction.
* **No earthworks.** The far thread builds from the world's spec, not the streamer's
  `Earthworks`; an edit is a record of what the player changed, and the player cannot reach beyond
  the window this field starts outside of. A save that carried far edits would not show them.
* **The slice's outer edge is open**, deliberately: a face whose neighbour has no census is not
  drawn, so the ring does not wall itself in.
* **A chunk crossing while the ring is building is coalesced, not interrupted.** The worker finishes
  its current chunk and the main thread schedules against the newest eye; the leading arc can trail
  the eye by the few seconds its generation takes.

# Tree

`voxel-client` lib tests: `far_terrain::` 4/4, `horizon::` 14/14, `scene::` 32/32; the material
golden images pass with no fixture regenerated; `the_shipped_view_is_not_a_black_screen` passes;
`light-at-a-pixel`'s named pixels are unchanged (`0.1981 · 0.1455` at the wall poses, the same four
decimals as PATCH-082). `shploingus` builds and runs with the far field drawing.
