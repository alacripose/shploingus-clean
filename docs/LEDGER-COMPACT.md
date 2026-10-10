# LEDGER (compact) — shploingus-clean

> **What this is.** A compact, complete companion to `docs/LEDGER.md`. Every entry
> **C-1 … C-165** from the detailed record is represented here; every claim is a
> compression of that file. The detailed ledger remains the **authority** (it is
> append-only, and the build gates read it mechanically — `voxel-package::record`,
> `compendium`, `migration`); this file is derived and must never be cited as a
> source in a declaration. When a number or a full argument matters, follow the
> entry number into `docs/LEDGER.md`; evidence files are named `logs/…` where they
> exist. **Never cite this file in place of the record.**
>
> **How it was built (2026-10-04, `freebuff-recovery` pass).** The recovery skill
> targets the Freebuff *desktop* app's SQLite store; this workspace uses the Freebuff
> **CLI**, which keeps no conversation store on disk (verified: only process-supervision
> temp dirs, no DB). So the recovery was done against the repository's own records:
> `docs/LEDGER.md` (11,720 lines, 160 entries), `logs/` (all raw evidence), and
> `docs/{HYPERGRILLING,TODO,STATUS,HANDOFF}.md`. Nothing was invented; numbers are
> kept where they are the point and dropped where they are detail.
>
> **How to read.** Entries are grouped by era, in order. Bold = the decision/claim;
> following sentences = what landed, key measured numbers, and what stayed owed.
> Status markers: **owed** (named, not done), **open** (awaiting a user ruling),
> **filed** (a ticket exists).

## 0 · What this tree is

The voxel game. 1 m voxels, 32³ chunks, signed 32-bit coordinates, first playable
band z = −16 … +47, 20-level addressing ladder; **z is up** since C-138. Headless
crates depend on `sha2` and nothing else; the world's determinism claim rests on a
small auditable arithmetic base (C-5, C-7). Crate layout follows boundaries that earn
independent compilation (`voxel-model`, `voxel-noise`, `voxel-shapers`, `voxel-mesh`,
`voxel-stream`, `voxel-appearance`, `voxel-work`, `voxel-client`, `voxel-ui`,
`voxel-instrument`, `voxel-package`, plus the `shploingus` bin). The client streams
`World::shipped()` = seed 2468, tick 0. Target dir `F:/shploingus-target`; offline
cargo; builds use `CARGO_INCREMENTAL=0` on this host.

## 1 · Standing rules (they bind every round)

* **Append-only**: a decision is superseded by a later entry, never rewritten.
* **Reference is not authority** (C-48/C-49, restated after being broken): outside
  trees inform reasons, never rule here.
* **Never test the client invisibly** (C-40): a green suite is not a client; a
  black world survived 461 passing tests and was found by an eye.
* **Run verification commands exactly as the docs write them, no filtering** (C-45
  addendum) — and the run must be readable by the user ("you read the run, or the
  run did not happen").
* **Refuse, never default** (C-8 and the whole refusal family): no clamping, no
  guessing; a missing thing is displayed by name.
* **Absence claims get read, not grepped** (C-88.3, C-90.2).
* **Catch-words**: "smallest" (C-90.3), "minimal" (C-84.1), "simpler" (C-97) — a
  slice offered is how the whole gets dropped. Whole-or-nothing for architectural work.
* **High agency is per-invocation and does not inherit** (C-56/C-58).
* **A test that cannot fail is a defect** (C-68.1; mutation-tested pins since).
* **The docs are the declaration** (R-62.2, C-63.2): a feature citing an anchor no
  authoritative document actually contains is refused by the build gate.
* Appearance is the user's (C-2/C-3 as confirmed); seasons are literal seasons.

---

## 2 · The record

### 2.1 Foundation — corrections found by measurement (2026-09-29, C-1…C-16)

**C-1 — this repo exists and is separate.** Sibling tree consuming the package API as a
crate boundary; escapes the old tree's module collisions and its 16,952-line
`diffusion-core`. Cost: two repos to keep coherent.

**C-2 — "256× sub-subvoxels" is an addressed procedural lattice, not stored.** Dense
storage would be 16.7 M samples/voxel. *(Open to correction; appearance is the user's.)*

**C-3 — NPR = Valve Source 2 shading over OpenPBR**, measured by the ≥3-luminance-band
criterion. *(Open to correction.)*

**C-4 — the rendering/perf mod class is one architecture**, not eleven ports:
instanced/indirect batches, culling, bounded caches.

**C-5 — crate boundaries, not features.** `ala-cities`' 2,728-line single binary
(read_hud touching 30 self fields) is the escaped failure.

**C-6 — settled first-target contract:** 1 m voxels, 32³ chunks, signed 32-bit,
z −16…+47 band, 20-level ladder. Not the long-term boundary.

**C-7 — headless crates depend on `sha2` and nothing else**; noise, octree and mesher
are written here.

**C-8 — materials are a registry with a conservation model**; unadmitted material = a
**refusal**, so the world cannot silently invent one.

**C-9 — `voxel-mesh` is written, not ported** (must fuse 4×4×4 subvoxel shape + octree
in one pass; no existing mesher does).

**C-10 — the 20-level justification was off 1000×** (16.8 km vs 16,777 km); ruling
kept, arithmetic pinned.

**C-11 — caves are a ridged field, not Worley** (1−worley produced 0 carvable cells;
replaced by Musgrave's 1−|n| fold).

**C-12 — boids separated toward each other** (sign inverted; test caught 0.4→0.24
convergence).

**C-13 — soil depth declares `MAX_SOIL_DEPTH_M = 3.0`** (Heimsath closed form → ∞ at
slope 0; declared boundary, not emergent).

**C-14 — rock varies per region: `CORRELATION_LENGTH_M = 220`** (bed-index-only
lithology made one rock across 12 km).

**C-15 — orographic sign fixed** (was making the lee side the wet one; rain-shadow test
measured 25 mm windward vs 2227 mm leeward).

**C-16 — a declared climate belt (`CLIMATE_BELT_C = 9.0`)** so the world has a climate
rather than a weather report (12 km once gave two biomes).

### 2.2 The layers (2026-09-29, C-17…C-34)

*Chapters: the mesh layer · measured performance · the instrument layer · the
appearance layer · the work layer · the client.*

**C-17 — visible mesh: one greedy pass over a 128³ raster, no LOD switch.** Uniform
regions merge to one rectangle *because* greedy cannot tell one material's plane from
another's — so no seam can exist between two paths. (The slab is 1,024 voxels, not
32,768; the wrong figure came from a test comment, corrected in C-51.)

**C-18 — the octree canonicalises on every write; the digest is the compression.**
A tree is a function of its contents, not its insertion history — same voxels, equal
digests.

**C-19 — the mesher's raster is padded by a whole voxel (4 subvoxels)**: 136³ raster,
no corner of the pad is a special case.

**C-20 — `Quad::uv` is chunk-local, not rectangle-local** (a renderer assuming
otherwise would offset every quad).

**C-21 — a raycast beginning inside matter reports `entry_face: None`** — no invented
plausible value.

**C-22 — transmissive materials come from the registry** (`OpticalClass`), never a
hardcoded id in the mesher.

**C-23 — a one-material padded raster has no faces, checked before lookup**; sky/deep
rock meshes in 1.17 ms instead of 21.0 ms.

**C-24 — `ColumnProfile`** makes a column's height-independent work happen once
(59.2 → 11.3 ms measured); per-voxel path is a one-line wrapper of the same code.

**C-25 — slots S-03/S-12/S-13/FC-2.16 built early** as instruments with complete gate
strings (sixth workspace member).

**C-26 — a band gauge cannot sit on a stock boundary**; refused at construction
(S-13 reports only a raw mass), found by measurement.

**C-27 — the two reserved interpretations confirmed**; OpenPBR values derived from the
material's own physics, the rest refused (asked as explicit questions).

**C-28 — the dense-detail figure in APPEARANCE §2 was low by exactly 64×.**
`DETAIL_SIDE = 256` is per **subvoxel** axis: 16,777,216 samples/subvoxel,
1,073,741,824/voxel, **2⁴⁵ ≈ 35 TB per chunk** at one byte a sample — which is a
better argument for "addressed, never stored" than the prose's 550 GB. Ruling kept,
arithmetic pinned (same shape as C-10). Also: the weathering scale is taken over
opaque materials only, so admitting a fluid cannot restyle every rock.

**C-29 — memory is a type here** (`owned_bytes`, `node_count`, `bytes` on octree/mesh/
chunk/resident) — the user's bound is enforced rather than quoted.

**C-30 — the streaming window is a box**: `radius_chunks: 8`, `vertical_chunks: 3`
(count 17×17×7); priority weights y ×4 because the axes are not equivalent.

**C-31 — F12 enforced at the engine level**; throughput measured (1 worker: 147 chunks
in 12.379 s, 84.2 ms/chunk) and the price of meshing from the generator alone named.

**C-32 — build tree + kache store move to F:, host must be MSVC** — two measured
environment blockers (C: at 298 MB free of 465 GB), not design preferences.

**C-33 — the client renders offscreen and the shader is held to the crate by a test**;
windowing lives only in the `shploingus` bin.

**C-34 — the FMA bar measured on a real frame: 2 bands derived, 8 of 8 stated.**

### 2.3 Design system and the first visible world (C-35…C-47)

**C-35 — the four-bucket rule is made countable, not reviewable.** `voxel-ui` has zero
dependencies; every constant carries its bucket (MD1 rule / MD1 example / platform
translation / project extension) and its source.

**C-36 — the window and the measurement share the constants** (`WINDOW`, `CEILING_BYTES`,
seed moved from the bin into `voxel_client::game` so tests read the same world).

**C-37 — the HUD is drawn, and legibility is a measurement on pixels** (glyphon +
rounded-rect distance field; MD1's 2 dp radius computed, not faked).

**C-38 — the world reaches the screen and it is black. Measured, and owned.** 21,526
quads drawn; the read-back found exactly two distinct pixels over 921,600; the report
was correct and the cause was not the renderer.

**C-39 — a sourced reflectance in the material record, and the world turns visible.**
Option (a) of three: reflectance as a physical property with a source, like density.
The values are the appearance area's; the table went to the user. *(This is the root of
C-140/C-143's whole-spectrum work.)*

**C-40 — the rule nothing but an eye could have found: never test the client invisibly.**
README blockquote, citing C-38 (461 passing tests, a black window).

**C-41 — four corrections supersede the first shaper attempt**, recorded rather than
quietly applied (the argument is the useful part).

**C-42 — shapes come from laws:** speleothems analytic; clastic shape as three
independent axes (Barrett 1980); carbonate/coral growth-form list.

**C-43 — quad-versus-triangle, answered:** coarse = greedy quads; fine = triangles
(the displaced surface); three distinct things had been read as one.

**C-44 — the 256 level reaches geometry** (`voxel-mesh::refine`), with two bounds:
`MAX_DISPLACEMENT_SUBVOXELS = 0.5` is about a **trim** (a bare amplitude past it is
refused with the word *carve*); a Slab claiming the addressed level was refused by the
audit.

**C-45 — Ponytail is not in use (standing).** Addendum, same day: the user reads the
run — verification commands are run as written, unfiltered, and the output must be
readable.

**C-46 — a measurement of the wrong world, caught by the client probe.** Terrain tests
used seed 31337 and were read as the shipped world; the truth for seed 2468 was
measured; a measurement must print its seed.

**C-47 — the seven reflectance values are approved as sourced**, no edits (USGS
Spectral Library v7; Lee et al. 2021; Massom et al. 2025).

### 2.4 Audits, rulings, and the block plan (C-48…C-65)

**C-48 — the audit before the debug views, and the reference's status.** The user's
order: go back through the record first; the reference is explicit that debug views in
the client show the debug systems (including world shapers).

**C-49 — the correction: other trees are not authority** (restated because it was
broken), plus the rulings: one append-only record, two views (H-8.1); packages first;
stage cadences (edits ×1, water ×4, erosion ×20, geology/climate at TICKS_PER_DAY —
provenance: proposed by the agent, adopted by the user).

**C-50 — the whole-ledger re-check.** One substantive finding: C-48's last paragraph
read as built work and none of it existed. Four form defects measured (only 10 of 49
entries findable by grep — bold 34, ### 5, ## 10). Provenance noted for the cadence
numbers.

**C-51 — the second re-check.** C-17's slab is 1,024 voxels (error came from the test's
own comment); `voxel-package` was a workspace member with **no ledger entry** (this is
that entry's argument); STATUS.md's dependency rows were stale; 8 ignored tests, not 7;
C-32's co-location reason was not in effect (kache says so every build).

**C-52 — the system is a build prerequisite, not an add-on** ("without the system as
outlined, we cannot compile a game"); the landing page's design surface is a real token
system (reference).

**C-53 — the landing is the UI's reference; its probes bind, its tokens stay
reference.** Chosen: both — one compile pass, two readers.

**C-54 — one chain for everything, namespaced** (H-11.7); the loader proves itself on
the shell's own arrangement package first (H-11.5); the shell API provides game panels
and toast notifications; depth evidence: the reference's default is no shadow at all.

**C-55 — the depth language was already declared** (elevation = brightness tint, not a
shadow; raised = lighter, `ELEV_STEP` 0.05, `RADIUS_DP` 2). Pane depth defers to the
arrangement package. The owed `voxel-package` entry written (§55.3).
### 2.5 Alpha invocation, the blocks, and the record (2026-09-30, C-56…C-65)

**C-56 — v1 alpha delivery invocation.** "Make the version 1 alpha package ship using
the material we have provided." Mission: integrated alpha, not another disconnected
demonstration; latest answers supersede stale open-question lists; coordinator context
bounded at 24,000 tokens. Evidence boundary stated: no shell module in `voxel-client`
yet, no `voxel-package` dependency in the game at invocation start.

**C-57 — successor-model readability is an alpha delivery requirement.** Checkpoint:
`voxel-package` foundation (inspect with retained rejections, canonical codec,
`alpha-shell.shspec`, build-script round-trip gate); 21 tests passed at that point.

**C-58 — high agency invocation: gix provenance, Laya linter/checker, envelope and
sealer.** `shploingus-game` read as reference only. Mechanisms carried: the signing
seam has no crypto in it (backend in a separate crate; `NoSigningBackend` refuses every
package, no feature flag); envelope and sealer as its own crate boundary.

**C-59 — the user's six rulings, and the block plan.** Provenance = our own
working-tree digest (C: gate checks declared features); envelope = full (pqc-binary-format
+ ML-DSA-65 across a crate boundary); unverified packages must load, so the load is
*described*, never silent; `.shspec` framework adopted with this tree's data structure;
linter = seam + structural judge with a real known-answer set; a real multi-subcommand
CLI; missing functions are evaluators for the building (`axis.rs` pattern).

**C-60 — the AWA3D effort loop's verification boundary.** Declared per-level budget,
recorded absence as a distinct renderable state; adds a fifth fail-closed rule to B3.

**C-61 — correction: the known-answer set was measuring invented vocabulary.** The
user's words: vocabulary like civilian was missing. The composition is **five parts** —
`recipe + résumé + civilian + governance + security` — and the recipe is one of them.
Calibration measured and reported.

**C-62 — second ruling round; B3b landed.** Five-part composition = **API view in the
right pane of the Mods window**; **the docs become the recipe**; policy chokepoint built
now (`policy.rs`, 23 closed actions, deny by default, missing matrix entry = compile
error); known-answer set to grow toward ~40 (owed at that point); `voxel-package` 83
tests.

**C-63 — the record becomes the declaration; calibration grown 13 → 53.** Every
recipe quotes the record's own language; `record.rs` holds the ten authoritative
documents and checks each feature's anchor against **the real text**; a design error of
the module's own (conflating missing witness with non-conformance) found by the check
refusing everything; 94 tests.

**C-64 — the UI spec contradicts the recipe schema already written.** `alpha-shell.shspec`
did not match UI-SPEC §2 (a `WindowSpec`); fixed against the record. **R-64.1**: the
recipe format is defined by this repository, versioned, and closed — documents pass
through it. R-64.2: each composition part maps to a document. The defect entry claims
the defect, not its repair.

**C-65 — the record compressed, and the crates that predate it measured.**
`compendium.rs` compresses ten documents into `shploingus.compendium/1` (verbatim
bodies, anchors checked); `census.rs` measures all 12 crates × 5 axes — **54 of 60
cells active**, six gaps each printed with its reason. Four defects found by *running*,
not reading. `portal_simulator.py` read for the first time; window-manager semantics
recorded. 148 `voxel-package` tests; build gate runs.

### 2.6 One chain, envelopes, tickets, Laya (C-66…C-84)

**C-66 — one chain, the v3 envelope, the ticket taxonomy read from the references.**
`chain.rs` (append-only, namespaced world/year/season/campaign; two real bugs found by
tests, incl. digest folding the *stored* digest); `envelope.rs` (v3 fields with counted
closed vocabularies; the bloat guard — the reference ledger was **81.5 % `details{}`**,
so the field was removed); `ticket.rs` (taxonomy ported, joined to this tree's
chokepoint). Workspace **731 passed**. Not done: console/log view.

**C-67 — this tree's wire format, decided here, and the ticket read-back.** `wire.rs`
port decisions, not either file; bloat structural (no open map anywhere); an in-game
ticket now compiles end to end (`round_trip.rs`). Four defects found by running, three
in code just written. **775 passed.**

**C-68 — the whole project consumed against the spec, and an axis that could not
fail.** D97 violation recorded (a Python measuring script; later ported). `census.rs`'s
Declared axis read the agent's own sentences and could not fail — replaced by reading
the spec's tables (`claimed_by_spec.rs`): 6 claimed / 6 unclaimed on the record, printed
every build. **780 passed.**

**C-69 — the human-readable API document, and the judge calibrated against it.** The
judge never writes prose (a judge with a sentence could be quoted); `apidoc.rs` renders
and checks against declarations; `WINDOW_FIELDS` centralised (the list had been typed
three times); six defects the calibration found, all kept; vocabulary derived, never
imagined. **786 passed.**

**C-70 — the axis mutation measured, D97's script ported, and the record found to have
no chain behind it.** The mutation reverted byte-identical; the Python bloat script
deleted and ported to Rust; **H-8.1's "one record, two views" is currently false** —
the chain is empty and the Markdown has 35 disagreements with it (filed, not smoothed).
**796 passed.**

**C-71 — the working-tree digest implemented; gix scoped to a condition rather than a
policy.** `provenance.rs`: COVERED = rs/md/toml/wgsl, SKIPPED = target/.git; unreadable
file → `not-measured` + path + OS error, never an empty digest. Correction to C-70.1's
evidence: `git diff` could not have proved the revert (repo unborn) — the same hazard
D107 names. **802 passed.**

**C-72 — what godplan is: the method and citations, not the document.** Its document is
generated, its generator absent, two copies disagree by 5,850 lines, and its preamble
self-contradicts. Ruling: keep the method; record the derived items as proposed, not
adopted.

**C-73 — the first record that outlives the process that wrote it.** `filing.rs`'s
append path built (every previous chain write went to a temp dir); `EvidenceGrade →
EvidenceOrigin` per ala-system's own type; two tests that measured the wrong thing
(the chain they constructed themselves) fixed; **the gap is 39 and stays 39** —
filing does not backfill; the segmenting rule for a second `.chain` is unruled.

**C-74 — the open-question list is 100 % wrong, and the record cannot mark a ruling.**
All 27 questions carry an Answer line; the reader counts a question mark, not a
decision. The module is right to refuse; the decision owed is *how the record marks a
thing decided*.

**C-75 — the answers audited: 56 of them, none names code.** Measured: 39 Answer
(user's words) + 17 Answer (selected). Five of nineteen feature rows are declared by a
ruling, three with no witness. The vocabulary for a ruling already exists
(`event_type`/`kind`); ruling: use the existing types, type the filing site, file the 27.

**C-76 — the writer can no longer spell a word the record does not admit.** Four fields
(`event_type`, `kind`, `state`, `control`) were `&str`; all four are now their enums, so
an undeclared word is a **compile error**, not a runtime refusal; the reader-side
corruption test now injects through the wire layer; the pin mutation-tested. **Owed:**
filing the 27.

**C-77 — 26 answers and 1 open ambiguity, filed on the chain.** All 27 filed through
the production path; measured result: **26 answered, 1 open (H-7.10)**; the chain is
`previous: not_evaluable` — append-only **without tamper evidence**, asserted so it can
never be mislabeled; split mutation-tested (injecting a Status line above H-8.1).

**C-78 — the pane tree, from the reference rather than the spec's summary.**
`portal_simulator.py`'s `split_pane` read properly; two bugs caught by the reference's
own tests; two refusals the reference does not have (layout refusals are not appearance
refusals — D-07's collision); `new` deliberately absent (an arrangement with no windows
describes nothing). One row of four active; the larger half (the gate) named.

**C-79 — the chain is hash-linked, and the linkage was never readable.**
`Linkage::render`/`parse` were not inverses; `append_event` validated the linkage and
ignored it; the fix is the `filing::linked_records` **chokepoint** taking
`UnlinkedRecord` (previous field removed), so a caller cannot opt out. Four fixtures
were asserting the weakness; all four failed, and that was the check working.

**C-80 — eight tickets on the board, and two reference features the shell lacks.**
Board read back off the chain (8 tickets, kinds asserted). The reference declares five
requirements, the shell had two; `shell_arrangement reads active` overclaims its row;
the craft floor (36 references, one a floor) read for the first time. The console / task
viewer / toast queue / panel are one API.

**C-81 — twelve tickets from the other projects' own READMEs.** Two are boundaries
rather than features (evidence naming nothing must be refused; filing must not write
outside the workspace); a check that was wrong was caught by the filing itself. **828
passed.**

**C-82 — four more tickets; the projection invents the gate.** `ticket.rs` built
`Gate { expectation: Expectation::None }` as a struct literal — the projection invented
its own gate; MSW's kernel named the both-halves rule. Nothing in the tree proves a
frame was drawn (all four GPU tests `#[ignore]`d). BLD-0015/0018/0025 rows; the user's
**Laya correction**: Laya is the entire decision layer, not the oracle alone (and the
oracle is its package instrument). Ten-minute test-timeout era noted: the board is
silent in a default test run.

**C-83 — one seam for every decision, and the oracle becomes an implementation of it.**
`audit(&decider)`: Subject = 60 axis subjects + 19 feature rows + the recipe
(`SUBJECT_COUNT = 80`); the deterministic decider answers by *calling* census/features/
recipe (no re-derivation); a missing instrument is not a pass; the gate prints the pass.
**833 passed, 8 ignored.**

**C-84 — the suite runs through Laya: the inversion, and the second pass.** The user
refused both offered halves of the inversion: **all five axes and all nineteen rows**
invert (readers report; the layer decides); `run(system, model)` returns a TwoPass and
prints both side by side; "the instrument could not answer" ≠ "no disagreement".
Three tests pin it. A live INT-0006 created by the entry and left visible; BLD-0025
filed (build the Rust model instrument so pass two decides instead of refusing).

### 2.7 The shell and the game (C-85…C-100, through 2026-10-02)

**C-85 — the shell runs in the client: the mount, the pinned layer, two defects the
window found.** The visible run is the only acceptance (C-40). Defect 1: the package
declared windows too short for the views they host; defect 2: one window's refusal took
the whole shell off the screen (a dock halved a pane; the fixed four-row panel no
longer fit). A px/dp units error fixed; `Role::Surface` admitted as a project extension
with bucket + source. **859 passed.**

**C-86 — the debug lanes: the mechanism, and the lanes on every window.** Lane hues are
`None` for every class (no theme declares a lane row; a test fails when one grows a
hue); HUD = the pinned subset of the same window set. The vector change view identified
as *not* a projected cuboid. **881 passed** (voxel-client 82).

**C-87 — the vector change view's subject: the boundary of what changed.** The user's
correction: the projection is wrong; the view is the cells the modification record
actually changed (with reason and material). A bow-tie corner bug fixed (24 edges → 12).
`LIFETIME_MS = 1_500` declared. Nothing draws it on screen yet. **892 passed.**

**C-88 — the deferred tick ladder, and the flash that a cache was causing.** The user's
report read: chunk flash was an absent chunk, not a changing one — the fix made the
user confirm "flash gone" by eye (C-40 satisfied). Pointer half named, not folded in.
Two absence claims corrected (the user was right twice): **an absence claim gets read,
not grepped**. `stages.rs`: Cadence + 16 stages, `SoilTransport` the sixteenth. The
tick does not advance yet. **897 passed, 8 ignored.**

**C-89 — the chain is not empty, and the reader feeding it kept 15 % of the record.**
`compendium::extract` ended an entry at the next heading of any level → 85 % dropped,
no check noticed. Migration imported the 54 pre-chain entries + the ruling = 55 events;
disagreements did not reach zero (numbering incompatibility filed, not decided).
**26 compendium tests.**

**C-90 — the smallest thing worth detecting, and the record that was never read.**
Three times a partial search became a confident claim about the whole record. The
user's correction: "smallest" is a detection instruction, not a permission. Two
standing rules added to `TODO.md §0`: **read the record, do not grep it**; the smallest
thing worth detecting. Record was 6,148 lines.

**C-91 — a material's surface pattern is a signed instance field on the subvoxel.**
A pattern is a field, not a file and not a mod; read at the addressed 256 level, never
held in a lattice. C-3's uncarried material half closes here (ruling only; no field
added). The meta-ball reference was searched for and **reported as zero**, not guessed.

**C-92 — the v3 envelope is this tree's; the reference audit called its drift a defect.**
The port's vocabulary equals the source file's value for value; the one surviving
finding: `Control` has no totality pin (17 `*_ARE_TOTAL` pins across 9 files; three in
`envelope.rs`). The audit skill was deleted — it was a transcription, and the record
already held all of it. **BLD-0018 landed** (console written, tested, called), teeth
proven by mutation.

**C-93 — the biome classifier cites Holdridge and is not fed Holdridge's inputs.**
Measured with `biome-drift.rs`: the biome flickers with the season, so it is not the
stable regional fact the biome projection needs. The fix (aggregate before classifying,
Holdridge's 0/30 °C clamps) is filed as **ECO-0001**, blocked on the user's ruling, not
the arithmetic. *(Fixed later in C-159.7's neighbourhood; the entry stands.)*

**C-94 — the onset layer exists, and a stage has a unit of work.** Unit = an octree
region at a declared level + one voxel of halo. Onset layer built (weather's onset field
kept; the tick is a *sample coordinate* there). Sky pass: **SKY_EXPOSURE = 24.0**
measured. Water ledger: derived storage first; **six reservoirs → three** (ocean,
surface, groundwater); a 876× `precipitation_years_per_tick` error caught by the
measurement; runoff moves 296× more water than rain delivers (recorded, not papered).

**C-95 — the camera stops floating: a body with gravity, a collider, a seed that has
ground.** `GAME_SEED` changed with a measured reason (a colliding body asks a question a
floating observer never had to). Four bugs the tests found; a test that could not fail
found by mutation (`SUBSTEP_M` raised 5× and nothing failed); **STEP_UP = 0.55 m cannot
fire on 1 m voxels** — named, not silently changed. GPU evidence captured; the floating
observer is retained as `laya_observation` (build-time vantage).

**C-96 — the world stops being a constant, and the HUD learns the body has a body.**
Seed per world (H-7.6) now reported, not a constant; body (ground/airborne, vertical
speed) and water (dry/submerged) rows added; the *package* was corrected, not the check,
when two rows overflowed the declared pane. World still pinned at tick 0.

**C-97 — "simpler" is caught every time.** The third catch-word, with a causal claim:
the word is the mechanism by which old decisions re-enter the clean room. Standing
consequence: **a change is the whole or nothing**; for architectural work the unit is
the system being repaired, not a slice.

**C-98 — the architecture contract exists; it is README.new.md's section, not a new
ruling.** `architecture/workspace.dsl`: one system, seven containers, five components
inside the decision layer, four views, rendered with `structurizr-rs`. One INFO is a
real shape (System Context has no player→system arrow). No person has looked at the
four images — not claimed. The user's correction (§98.8): the decision layer was drawn
beside the path and **missing its five named parts**.

**C-99 — Laya is wired into the game, and the agent never read the player's filings.**
Before: 132 records all written by tests. After: the pass runs at world-open on the
game's clock; the game writes the queue, not a test. **The correction that matters
more than the wire (§99.4): the agent killed the player's game twice with `taskkill`
without reading what it had recorded** — five tickets the player filed were found only
afterwards. Standing rule: the board is read before any run the agent started is
stopped.

**C-100 — the agent's inbox: the recipes the game gives, written by Laya.** The agent
authors nothing; the game writes recipes into `logs/agent-inbox/` at world-open (read
back through `NotificationStream::open_at`, handed over at mount). Truncated file =
refusal. **Owed: digest-addressed, not name-addressed** (`packages/BLD-0207.shpkg` is
one name for one ticket; a ticket whose package declares nothing can never be
recompiled).

### 2.8 Laya's citizens, the shell port, the field plane (C-101…C-120, through 2026-10-03)

**C-101 — the Laya observer is the only citizen; the city half stays reference.**
`gov/mod.rs` was an island (1,823 lines, nothing outside names it); reduced to the
record side; the city half moved to `city_reference.rs`, not declared, not deleted.
Measured: `voxel-model --lib` green.

**C-102 — Laya recalibrated: 20 readings, both verdicts, checked by the judge's own
rule.** 10 Belongs / 10 NotBelongs, every row on this tree's own sentences;
`every_known_answer_agrees_with_the_vocabulary_rule` re-derives each expectation with
`terms()` so a row added wrong fails.

**C-103 — the gate's proofs are minted from the citizen's rows, read back from the
commit point, graded on `axis.rs`.** Two notification tests added; **still 0 cleared**:
no production caller commits a governed act yet (`GovernedWorld::apply_action` /
`execute_plan` have no caller).

**C-104 — the ledger is filed as tickets; the game catches the board; Laya grades the
citations.** Every `## C-` entry filed as a `[CSE]` ticket (question = the heading,
answer = the citation); measured callout: **455 ticket rows — 64 read back, 473 named
at the end, 314 CSE cases**; the layer reads tickets as a second arm (not a widened
rule). 244 broken rows are the old tiler's, kept as evidence; repair is a regrade or a
retirement, never a retraction (append-only).

**C-105 — the board-wide ticket evaluator: 473 open tickets compiled, the ledger's 314
cases handed to the agent.** The reduction removed: a ticket filed in another world
stays in scope. **Does not yet retire**: the compile is the work the agent starts from.

**C-106 — the calibration is the tree's whole vocabulary.** The judge at **104 readings
over 52 domains** (belongs 52 / not_belongs 52 / no_reading 0); the package instrument
`oracle` at **103**; each row quotes its system's own declaration.

**C-107 — the earliest entries were not tickets: the filer grepped one heading level.**
The record holds entries in three heading shapes (bold / `###` / `##`); the filer now
reads all three. Measured: 529,546 bytes, 8,093 lines, **106 C entries → 106 tickets**
(34 bold + 5 `###` + 67 `##`). Old tiler debris (214 singly-filed rows) named.

**C-108 — the camera could not turn once the lock landed: look is a device event.**
`Locked` on this platform does not deliver the motion; the fix treats look as a device
event (raw motion), not cursor deltas. The eye check (C-40) not yet made.

**C-109 — the toast is the shell's own surface, one at a time, with the timer
visible.** Nothing drew a toast before; queue semantics (one rendering per record,
tick-timed, split and recorded) landed. **1288 passed, 26 ignored.** Eye check owed.

**C-110 — the shell is a port, not a derivation (user ruling).** The port's inventory
taken from `portal_simulator.py`'s own class list; what survives because it is *ruled*
(the one-writer queue, one API both kinds, persistent split, toast semantics); the apps
not ported named. Porting order set: window machinery → desktop/taskbar/theme →
pointer/console → toast's final form.

**C-111 — the port plan is a document; the first module landed; D-1 and D-2 ruled.**
`docs/SHELL-PORT.md` (module map, landing order, gates, retirement list) landed before
the first module. `titlebar.rs` ports `WindowElement`/`ControlButtonGroupElement`/
`TitleBarElement` line-ranged. D-1: control size floor = MD1's compact pointer
translation (48 dp is a touch target). D-2: one top surface.

**C-112 — the pane tree's split was faithful in its description, not its code.**
`arrangement.rs`'s split was unfaithful to the reference's `split_pane`; replaced by
`split_beside` (find the splitter where the target is a direct child; orientation
agrees → insert at idx ± 1). `pane.rs` and `splitter.rs` land with their reference line
ranges.

**C-113 — the drop preview and the window's lifecycle.** `drop.rs` (outer 24 dp edge
band first, then the pane's nearest zone; a zone name refuses like a theme role) and
`window.rs`'s lifecycle half (new = one pane holding the window's own id; split, dock
with the reference's Before reversal). Composite drawing still owed.

**C-114 — the window's composite and its mouse machine; step 1 complete.** The
composite is one arithmetic (`composite(spec, rect)` hands out title-bar slot and
content container; every rect resolves against that pair — §5's one-arithmetic rule
made structural); `press` is one verdict per event (resize strips → title bar → seam →
pane). **Workspace 1358 / voxel-client 282.**

**C-115 — step 2 landed, and the desktop draws nothing: the world is the background.**
The user's ruling: "the desktop background over the game would be dumb". `desktop.rs`
(registry rules), `taskbar.rs` (data, not a surface — D-2), `theme.rs` (the coverage a
theme must state, not the 85 colours). **Workspace 1379.**

**C-116 — the pointer answers with the port's verdicts; the derived layout helpers
retire.** `drop::zone_at` was reading the desk where the reference reads the window
(widget-local) — fixed with a regression test. One construction for `elements_at` and
`press_at`; `DragState::preview` the only retained state. `layout_panes`/`pane_at`/
`pane_rect_of` deleted; wheel bug found while wiring. **Workspace 1386.**

**C-117 — the shapers are *not* tick-independent: the world is frozen, not timeless.**
Measured: **1985 of 5445 voxels differ** when the spec tick moves (tick 0 vs 24,000);
`stage_seed` mixes the tick as a hash input, so tick N is an independent world.
Corrected in the two files that claimed independence. *(Precondition for C-159.8's
fix.)*

**C-118 — the double-click source lands in the bin; the census's untested set closes.**
`DOUBLE_CLICK_INTERVAL_MS = 500` + slop declared where the platform is translated;
`shploingus` gains its first `#[test]`, so the census's one finding closes — and it
stays a finding, not a value.

**C-119 — the sixteen stages measured against the clock; the foundation under them is
the reference's cubic world core.** `stage-clocks.rs`: one row per stage, cadence read
from the stage. Foundation finding: ala-cities' `world/` core (25,747 lines across
world/materials/matter) is the reference for a cubic world. Change view moved to the
shell's witness.

**C-120 — the chunk foundation cut onto the field plane; the pointer lock centres
before it grabs.** `field_sampler.rs` ported: `FieldId` (19-name roster), `Unit`,
resolution levels. Pointer lock now centres first, so the camera does not jump when
grabbed. **Shapers 235 / stream 29.** Vertical axis y-vs-z named as the user's ruling
(later C-138: z is up).
### 2.9 Process runtime, materials library, the sweep (2026-10-03, C-121…C-137)

**C-121 — the sixteen stages run as processes; an unadmitted body refuses by name.**
`process.rs` ported onto this tree's stage roster; owed bodies named in `owed_body`
(Fields, Erosion, Caves, …) — nothing invented to fill them. **Shapers 252 passed.**
Still user-only: the vertical axis and the mouse-lock eye check.

**C-122 — the starter materials are a fabrication; the material library is *derived*.**
`column::materials`' eight ids are a placeholder, not authority; the roster must not
pick materials, it must prepare the seam the library fills. `shape_at`/`build_chunk`
wiring held until the seam is decided (C-123.3, C-125.3).

**C-123 — the process runtime and the tick scheduler share one cadence ladder.**
Tie test across every rung boundary; **voxel-work 52 passed**.

**C-124 — the material system scoped end to end.** `docs/MATERIALS.md`: exhaustive
inventory of every material artifact across this tree and three references; Round 18's
eight decisions (H-18.1–H-18.8) left open for the user. No code changed.

**C-125 — the materials grilling, Rounds 18–24.** Settled: library home = this tree,
Rust generator, digest-gated, no runtime parse (H-18.1); no floats; patterns are instance
fields; the fabricated eight named. Paused at the user's fatigue; fluids resume on their
call. Shaper wiring stays held (C-123.3).

**C-126 — the Tekkit Classic pull.** Thirteen source trees added (27 total under
`F:/shploingus-mod-sources/`), HEAD-pinned in `_harvest/clone-hashes.txt`; the IC2 jar
avenue verified (source closed, mirror packaging MIT); open passes listed.

**C-127 — no Minecraft progression, and the double-cover rule.** "Leaving diamond to do
real diamond things": the game does not follow vanilla tiers; gear rows wait for a
real-world source. Doc edits only.

**C-128 — the power-model lead found: the original tree's S-01 kinetic-power ruling.**
The "asked before" resolves to the modding-ledger-tech slot; recorded as a best guess
awaiting the user, not a ruling.

**C-129 — sweep every reference before the Rust invention.** The user's directive
("did you go over all the mods? you might want to do that before rust invention")
recorded as a working ruling: no Rust until the sweep is conclusive by accounting.

**C-130 — technologies are swept too; progression is a cellular automata.** The mods
propose their own tech trees, so the progression system needs the sweep's whole
vocabulary; CA sketch follows the sweep.

**C-131 — the sweep's Stage A completes (every cloned tree read).** Railcraft values
measured (its API submodule cloned at its exact pin); every artifact is a measured file
under `_harvest/`. No `crates/` file touched.

**C-132 — the technology sweep runs last (user correction).** Create addons reference
the legacy tech trees, so tech dedupes after all mods are read.

**C-133 — the sweep completes: Stage B swept, Stage C accounted.** DynamicTrees
(species data-driven), DynamicTreesPlus (8 species), and the per-URL verdicts; pins and
clone logs under `_harvest/`.

**C-134 — the technology pass: one pass, deduped, with the CA progression sketch.**
`MODS-MATERIALS.md`'s §8 re-derived as the final pass: **41 deduped rule rows** across
the families; the old provisional first-union tables replaced.

**C-135 — the materials library, step 1: schema, gate, generator, digest gate.**
`materials/schema.rs`: ten families, eight hue anchors, body/edge/deep with exact
deltas (0, +1/10, −9/50); the Rust generator compiles tables from declarations, and the
digest gate refuses a stale table. Gate: **2 tiers, 21 substances, 11 processes**;
density is a *property* of every row (fixed in the gate, not papered over).

**C-136 — the return path: why the materials pass exists.** The user's thread recorded:
the pass came from fixing the shell, which came from the materials rabbit hole — and
the reason is to **render the committed materials and repair the chunk system**
(measured against the column mandate), not to do materials as an end in itself.

**C-137 — ledger read-back audit: C-131…C-136 checked against raw evidence.** One
figure corrected (C-134's count; the roster uses 45, not 43 for the MFR rows); the
`create-processes.md` recount note added so the number cannot drift again.

### 2.10 The visible world: z-up, light, materials, movement (2026-10-04, C-138…C-151)

**C-138 — the vertical-axis ruling: z is up.** "That was already ruled in the past
project." Binding for all new work; consumers scheduled (the seam and the return's
chunk repair), never a retrofit of the incomplete generator.

**C-139 — correction: family × hue × level is the icon builder's, not the voxel game's.**
Re-scopes the materials step: no family×hue identity wiring; the role/level rows stay.

**C-140 — the reflectance serves the sun/moon light model.** The user's direction:
reflectance exists because the world is lit by the Sun (and the Moon reflecting it at
night, never properly wired); materials get material-accurate colours, and the whole
spectrum is kept. No code this entry — the plan.

**C-141 — the answer set: composition drives appearance, the whole spectrum is kept,
bedrock, S-01.** Asked and answered: the regolith sample, the bedrock sample, the band
rule, and the S-01 best guess; whole-spectrum harvester becomes the derived view.

**C-142 — the dominoes: materials loop → rendering wiring → column-based chunks →
atmosphere/fluidics.** The user's framing, with the hard constraint: *"if the chunks
keep the incorrect columns shit then you will have failed."*

**C-143 — the sampled appearance lands: whole spectra, exact derivation, the gate.**
`materials/spectra/sources.tsv` + the USGS splib07a files it pins (7 sample spectra +
2 channel grids); the derivation is exact and the digest gate passes on normal builds.
Open soft calls: the regolith stand-in (dry playa, flagged) and the ICE path (a snow
stand-in or wait for a real ice spectrum).

**C-144 — the regeneration door, and TFC's whole rock set in the declaration.** A
workflow defect found and fixed: the stale-table gate also blocked building the
generator itself — the documented regeneration door (`materials-regeneration`) closes
it. Content pass continues whole; no scope cuts.

**C-145 — TFC's ore roster declared as minerals (Ore.java/Metal.java read back).**
32 ores (12 graded), 27 metals + the steel chain; the gate now reads **2 tiers, 70
substances, 11 processes** (21 seed rows; the test comment now says 21).

**C-146 — the break/place freeze: the pool's read lock, measured and fixed.** The
user's pivot: edits froze because a read lock spanned the column work. After the fix,
busy-pool edits 25.8–54.2 µs — idle baseline (43.4 µs) within noise. Worker cost
(148–164 ms per `build_chunk`) **recorded, not cut**; field caching named as the
candidate.

**C-147 — the promotion the pool promised, the duplicate build it was paying for, and
two window defects.** The tree did not compile (first finding); `Streamer::pump` and
the promotion state fixed; a duplicate chunk build removed; two window defects fixed
for the OBS capture. **voxel-work 55 passed.** Window fixes need the user's eyes.

**C-148 — the field-plane memoization: C-146.3's candidate, done and measured.**
Consumer-side only (no shapers change): `ColumnMemo` in `world_source.rs`; nine paired
runs — the *build* is barely moved and the win is small on a noisy instrument.
**Stream 32 passed.** The remaining cost (per-voxel `stratum_at`, the material rule) is
untouched and not proposed.

**C-149 — F4: the meshes' greedy quads outlined, hue by depth.** `quad_edge_bytes()`
takes the four border edges, never the diagonal (the view's claim is the merged
rectangle); white ink was "really hard to see", so hue by depth is the answer. **Client
336 passed.** The look still needs the user's eyes.

**C-150 — the Squake movement, camera roll, and CameraOverhaul's effects.** `motion.rs`
replaced with the resquake model (reference read; config caps inert at their own
thresholds); camera roll added. **Client 360 passed** (28 motion tests). The feel needs
the user's hands; no HUD readout yet.

**C-151 — the crouch lowers the eye.** The user, after playing: "Feels great, just the
camera doesn't shift downwards when you crouch" → `SNEAK_EYE_HEIGHT = 1.27 m`; the feet
do not move, only the eye. Collider shrink not ruled.

### 2.11 Fluidics → sky → water → biome materials → trees (2026-10-04, C-152…C-159)

**C-152 — the fluidics first: the user's redirect, the cloud reference, the plan.**
The materials content pass was reordered: fluidics → atmosphere → shaper ticking. Read
(not invented): ala-cities' fluids roster and matter model; `docs/clouds_and_atmospheric_formation.pdf`
recorded in-text (saturation, LCL, parcel). Landed: six conservation families with a
tolerance class each (`NoCrossing` default; pressure/flow `Gating(1/1000)`); porosity
alone is not permeability.

**C-153 — the lattice lands: the column is superseded, the PDF is the model, 256-level
fill is its display.** Column deleted, not deprecated (no consumer outside its tests);
`fluidics.rs`: 6-neighbour CA over `BTreeMap<VoxelPos, Cell>` — Sourced properties,
Curve, Retention, Fluid, PorousMaterial; the books close exactly. Two bugs found by
tests: flow direction inverted; **`Ratio` saturates, so the transfer plan is
`Rational`** (a saturating type is not the home for a plan that must refuse).

**C-154 — water in the world: the fluid answers, the edit refuses, rain crosses the
weather family's tolerance.** `RefusalKind::FluidIsNotABlock` — the hand cannot delete
water (the filed defect). Calibration measured (seed 2468): max lake depth 2.036 m;
44 columns deeper than 0.5 m, 13 deeper than 1.0 m; 179 of 1760 ocean columns hold a
flooded karst void; `fluid_at` 0.340 ms per call (edit-time, not per-frame). Three
stale appearance greys and the rain mass path fixed. **Shapers 283 passed.**

**C-155 — the sky is a gradient and the bodies are missing: the atmosphere-rendering
directive.** "Change that dumbass sky triangle into an actual sky" + add the Sun and
Moon as previously specced. True today: the scattering physics sits on the CPU
(Rayleigh/Bodhaine, Mie) and the dome is 1-D — the sky is a gradient because the
triangle cannot vary with the camera; the model half is C-157, the drawing half C-158.

**C-156 — clouds are emergent: the parcel, the LCL, and the 256-level rain interface.**
`clouds.rs`: surface air, parcel with declared night cooling, saturation → LCL. Measured
(seed 2468, 6561 columns + a 100 km sweep): LCL noon min 372 / mean 1029 / max 2654 m;
midnight min 0 / mean 519 / max 2144 m. Finding next door: the terrain is flat by
relief, not by a compressed world (relief-profile measured; the terrain fix is a
write_todo, separate). **Shapers 292 passed.**

**C-157 — the sky's angle is spherical now (the model half of C-155).** The old
solution formed the sun–view angle as a product of sines; replaced by
`cos γ = sin v·sin s + cos v·cos s·cos Δφ` — the one place the angle is formed. A draft
test assertion was corrected rather than the model bent to it. **Appearance 89 passed.**

**C-158 — the sky is directional in the client: the dome table, the world-ray shader,
exposure re-measured.** 2-D dome (64 × 64 = 4096 entries), camera yaw in the uniform,
world-ray direction per vertex; `SKY_EXPOSURE` re-measured **24 → 12**. The recorder's
two tests raced a process-wide book (found because a test read 2 where it asserted 1).
The new report recorded verbatim (the "atmospheric blunder" walking artefact);
**client 362 passed.** Owed: pin the report to a term (done in C-159), then Sun/Moon
discs.

**C-159 — the fluid surface reflects the directional sky, and the world turns
biome-driven and inhabited.** Six entries in one:

* **159.1–159.6 — the walking report pinned; water stops being a matte sheet.** The
  report (put to the user; answer: "see-through water", trigger "not sure yet") was
  reproduced in code: water had **no view-dependent reflection at all** (only the
  ambient term could reach a transmissive surface). Landed: `WATER_IOR = 1.333` and
  `WATER_F0` derived from it (≈ 0.0204, never quoted separately); `fluid.rs`
  (`reflected_ray`, `schlick`, `surface_radiance` = F·sky + (1−F)·body); the shader
  mixes the reflected dome; CPU and GPU agree to **2.5e-10**; a GPU test shows the
  surface tracks the Sun (0.04881 toward vs 0.00820 away). Three ignored GPU tests
  diagnosed as three unrelated stale claims and fixed: stale **exposure 8.0 → 4.0**
  (re-measured: mean 0.378, max 0.717 with headroom; a power of two so exactness
  tests still hold); the HUD legibility guard re-measured (15 runs, contrast 5.94:1);
  the mount handover test now asserts the real property (every open ticket has a
  recipe; inbox holds 515 with 214 historical receipts). **Appearance 93; client 364,
  11 ignored; water GPU 1 passed.**
* **159.7 — the climate envelope was applying latitude twice; the world's declared
  45° was the Arctic.** `base_temperature_c = 15.0` + a ±45·sin²φ span put 45° at
  −7.5 °C; census read tundra/boreal/alpine only, warmest biotemperature 2.81 °C.
  Fixed by calibration: **`LATITUDE_SPAN_C = 36.0`, `base_temperature_c = 26.0`**
  (equator 26, 45° ≈ +8; pole −10 stated, not fitted). Re-measured: **8 of 11
  formations, warmest 16.677 °C.** Named as owed: MAP still reaches **29,510 mm**
  (needs the same calibration) and the belt wavelength is still ~400 m (regional belts
  owed). Tests 293.
* **159.8 — the world stopped reshuffling: every field that is a *field* got a
  stationary seed.** The user's ruling: "these 10 minute timeouts can't continue…".
  `WorldSpec::stationary_stage_seed` applied to terrain, geology, caves, tectonics,
  climate, weather, erosion, deltas: the probe went **270 of 270 → 0 of 5445** voxels
  changing when the spec tick moves. The client still streams tick 0; wiring the
  scheduler is the next step on that row.
* **159.9 — GAIA.md read: what Gaia needs.** It acts on living things via a **soil
  account** and **fertility** (absent today) and its rules are per **species** and per
  **region** with carrying capacity (both absent); its log is bounded per species ×
  region per cadence tick. Order stands: sub-biomes → species/regions → scheduler →
  Gaia.
* **159.10 — biome-driven surface materials: the ground stopped being one pale
  playa.** `materials::surface_for_biome` is the one mapping (Desert→SAND,
  Grassland→GRASS, Tundra|Scrub→DRY_GRASS, Boreal→CONIFER, forests→LEAF,
  Alpine|PolarDesert→REGOLITH); five new USGS samples; registry admits **12** (13
  constants, AIR never admitted). The biome read had to be made affordable first:
  `holdridge_inputs` hoisted the x/z terms and the seasonal scalars —
  **202.238 µs → 5.605 µs per column** (36×), pinned by a parity test. Both material
  paths read the biome; the field plane gained its `Climate` producer (roster 5 → 6).
  Census: spawn window `{dry grass 1979, grass 925, leaf 780, sand 314, conifer 205,
  regolith 16, metamorphic 3, water 3}`; whole ±6 km world eight covers. Tests:
  shapers 304, stream 32, client 364, work 57.
* **159.11 — trees: flora is a function of position, and the forests stand.**
  `flora.rs`: 8 m cell lattice, hash → candidate, jittered base column, formation gate
  (forests 1.0; grassland/scrub 0.18; tundra 0.04; alpine/desert/polar 0); conifer =
  trunk 7..12 + cone skirt; broadleaf = trunk 4..7 + flattened sphere; trunks WOOD,
  canopies CONIFER/LEAF. Tick-free (`stationary_stage_seed("flora")`); memoised per
  `(fingerprint, cell)` (200,000-cell bound, a cache of a pure function, pinned). Wood
  = USGS record 21247 (`gray_pine_branch`), material id 13, registry admits **13**
  (14 constants). Measured: ±512 m → **1014 conifer + 1098 broadleaf**; ±6 km →
  **168,430 + 108,027**; a real tree walked through both material paths and asserted
  equal; chunk build 128–149 ms (inside the previous noise). **`shploingus.exe`
  rebuilt 21:22** (`F:/shploingus-target/debug/shploingus.exe`).

* **160 — the sub-biome layer: terrain expresses the formation.** `habitat.rs`: five classes —
  steep (`slope ≥ 0.5`), montane (`≥ 0.45·relief`, below the alpine override 0.85), wet
  (`moisture ≥ 0.55 && slope ≤ 0.12`), dry (`moisture ≤ 0.35 || slope ≥ 0.30`), lowland — pure in
  `(surface, slope, moisture)`, precedence steep → montane → wet → dry → lowland.
  `materials::surface_of(biome, habitat)`: wet `dry grass → grass`; dry `grass|leaf → dry grass`;
  montane `grass → dry grass`, `leaf → conifer`, `sand → regolith`; steep → `None` → the
  lithology's rock (both material paths agree; the equivalence test is unchanged). Flora: steep
  0 trees; dry ×0.35; montane ×0.7 + conifer. Census (seed 2468): spawn window `{lowland 3474,
  wet 686, dry 65}`; whole ±6 km `{lowland 1176, wet 335, dry 85, montane 59, steep 26}` — no
  dead arm. Cover shift (spawn): `dry grass 1979→1702`, `grass 925→1213`, `leaf 780→769`; trees:
  window 1010 conifer + 1093 broadleaf, world **245,272** (was 276,457 — recorded as measured; a
  per-habitat count is the named door if it matters). `tick-over-world` still **0/5445**. Owed
  inside the layer: aspect (`Radiation`), drainage (`Groundwater`). Tests: shapers **311**,
  stream **32**, client **364 + 11 ignored**, work **57**; `shploingus.exe` rebuilt **22:42**.

* **161 — the material's own pattern, drawn where the greedy mesh has no geometry.** A
  greedy-merged quad carries one material and no coordinates, so the block grid it spans is
  invisible; the 256 lattice is **addressed, never stored**, so a fragment can read its block's own
  detail with no extra quad. `pattern.rs` is the model and `shading.wgsl` its transcription, held
  together by `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry` — every
  pixel of a stated probe compared against the crate's own `f64` answer on the GPU.
* **162 — the eye-check's corrections: world-anchored, per-material shaped.** The address is the
  **world** subvoxel (`floor(p / 0.25 m)`), not the block-local index — a block-local address is
  the same number in every block and stamped one pattern into all of them (the visible tiling).
  The eye's two halves (`eye_phase`, `eye_voxel`) let an eye-relative fragment address the world (a
  12-byte write for a 16-byte field had shifted `rates` to zero and resolved the 256 level at
  200 m). Every material **shapes** the same white noise (`materials::SurfacePattern` — speckle /
  foliage / bark / brushed: a stencil on the address and a curve on the value).
* **163 — the 256 base is 16 per subvoxel axis, the ladder is bounded, and the footprint is the
  eye's own geometry.** The user's correction: *"256 was supposed to be 16x16x16 like the 64 base
  was 4x4x4 lol"* — `DETAIL_SIDE = 16`, so a subvoxel face carries 16×16 = 256 samples and the
  lattice's cell is **15.6 mm**; and *"we definitley don't need fluidics to be the 256 actual
  base"* — fluids keep their own constant, `FILL_SIDE = 64` levels per block axis (one per
  15.6 mm), with the scale ruling attached (*"as small as individual 32x pixels (not 256x because
  it'd waste performance...)"*). The placeholder 64-level tone is retired (C-122 §122.1: the 64
  level is the library's **stored slots**); the pattern is one field read in octaves
  (`15.6 mm · 2^j`, to 4 m at `OCTAVE_TOP`), fBm-weighted and normalised, each rung smoothstepped
  to zero at **four pixels a cell**. The eye liked the fade and asked to keep it (*"there should
  be a way to fix the flickering and still keep it"*), so the **input** changed: `fwidth` — a
  per-2×2-quad derivative, which boiled on angled faces and painted the quad's own diagonal as a
  visible line — is replaced by `eye_footprint`: distance × one pixel's angle ÷ cos(face), floored
  at `FACE_COS_FLOOR`. The pixel's angle travels as `View.screen` (128 → **144 bytes**). No shader
  in the workspace computes a derivative now. The numbers ledger's dense rows moved with the base
  (**4,096** per subvoxel, **262,144** per voxel); the documents keep the old prose (C-38) and the
  gate names them.
* **164 — the drawn scale is the user's own: 32 samples per block axis.** `OCTAVE_FLOOR = 1`,
  `MIN_FEATURE_M = 31.25 mm`; the addressed 15.6 mm cell is never drawn. `FILL_SIDE` followed to
  **32** (one level per 31.25 mm). And a real bug fell out: the shaper's stencil and the rung's
  quantiser were both masks, which compose to the *max*, so bark's stretch vanished above the
  finest rung; the stencil is now a **shift applied before the rung quantises**, so a tree reads
  as bark at every distance. Tests: pattern 11, shapers 312; GPU green (worst 1.9e-6).
* **165 — the pattern tints, it does not wash.** The palette was measured (regolith 0.49/0.36/0.22
  against wood 0.11/0.06/0.04) and was never the problem: `with_detail` moved each albedo channel
  one-sided toward white, so every material carried the same pale speckle. Albedo is now a
  multiplicative tint (`PATTERN_TINT = 0.35`, hue preserved). Appearance 94; GPU worst 4.4e-7.
  And settings stopped being pinned by tests: the toast-duration tests derive the tick count from
  `toast_ms() / 1000 × TICK_HZ` rather than stating 6 s (notifications 9/0 at the user's 500 ms).
  Skills installed on request: `webgpu`, `rust-skills`, `webgpu-threejs-tsl`, `typegpu`.

---

## 3 · Current state (as of C-163, 2026-10-05)

* **The world the client streams**: seed 2468, tick 0 (the scheduler still has no
  production consumer). Its declared 45° latitude now carries **8 of 11 Holdridge
  formations**; the surface material is the biome's own sampled USGS cover,
  **expressed through the terrain habitat (C-160)**; trees stand in both material paths; water reflects the same directional sky the pass draws;
  the sky is a 2-D dome with the sun's angle; clouds' model and rain's 256-level
  interface exist; the body has gravity, collision, crouch and Squake movement.
* **Verification state**: per-crate suites green at C-163 (client **375 + 12 ignored**,
  including the pattern module's 11; shapers **312 + 1 ignored**; appearance **94**; model
  **293**; `voxel-package` **274 passed, 0 failed, 12 ignored** after the C-163 append);
  the GPU pattern obligation agrees **2.0e-7** over 260 100 pixels a stage (bound 5e-3) and
  is *equal* to a no-field render past the ladder's ceiling;
  workspace green after C-159.5 (1580/0, log c159-workspace-test-3.txt); GPU water
  agreement 2.5e-10; `tick-over-world` 0/5445; exposure 4.0; HUD contrast 5.94:1.
  Known limitations: the GPU tests are `#[ignore]`d on this host (run explicitly);
  long workspace runs are avoided by user ruling; `rg` broken → `grep -rn`;
  `CARGO_INCREMENTAL=0` required for builds.
* **Visibly missing, in the user's terms**: the Sun and Moon discs/halos; clouds drawn
  in the sky and falling-rain animation; ice's own dielectric/spectrum;
  regional biome belts (the 400 m patchwork); MAP calibration; ocean waves and swimming; a placement picker; HUD readouts for movement
  effects; the debug/F4 rainbow outline's eye check.

## 4 · Owed, consolidated (as of C-163; names, not promises)

* **Next on the user's own order**: scheduler driving the ticked world (tick-moving
  stages need persistence first) → species/regions → Gaia (soil account, fertility,
  carrying capacity per region; C-159.9). The sub-biome layer landed at C-160; aspect
  (`Radiation`) and drainage (`Groundwater`) are its owed doors.
* **Sky/light**: Sun and Moon discs/halos with the Moon's phase; clouds in the sky;
  the falling-rain animation; ice's dielectric + sourced spectrum (C-159.6).
* **Climate**: MAP calibration (29,510 mm is implausible; Earth's wettest ≈ 11 km);
  regional belt wavelength (C-159.7).
* **Fluids**: conductors — pipes as carriers, cracks as their own hydraulic domain,
  a sourced conductivity `k` before real materials conduct; surface-water reservoir
  drains/evaporation sink (C-94.8, C-154.6).
* **Materials**: the metals tranche (27 metals + steel chain), fluids roster rows, GT
  set, mods feature rows (C-135.3, C-145.3); the library's own base patterns
  (`MATERIALS.md` 4.9/4.10) are the owed content behind C-163's ladder; **the NPR half of
  the OpenPBR/NPR equation** — albedo, tint mask, tint colour, scrolling textures/UVs/
  normals, emissive colours/masks/strength, metallic and roughness maps, detail masks, NPR
  transmissives, subsurface scattering, rim lighting, NPR outlines (the user's channel list,
  Deadlock / Source-2's PBR shader, the alacripose-landing example; C-163).
* **Governance**: ticket evaluator compiles but does not retire (C-105.4); 244 broken
  board rows need a regrade/retirement (C-104.7); BLD-0025 model instrument; BLD-0026
  two passes in a window (C-84.6, C-92.6); the 39 pre-chain entries gap and the second
  `.chain` segmenting rule (C-73.4/.5).
* **Client**: the world still pinned at tick 0 (C-159.8); portal.rs and the slot
  identity rule (C-116.6); movement effects need the user's hands and a HUD readout
  (C-150.3); F4 rainbow outline unverified by eye (C-149); toast/debug eye checks
  (C-109/.86); `STEP_UP = 0.55 m` fires only on sub-voxel relief (C-95.5); the pattern's
  obliquity term flattens faces seen at grazing incidence — the eye reads it as *"the pixels
  merge... horizontally"*, and **per-axis (anisotropic) rung selection** is the named next
  mechanism (C-163); the sky's turbidity follows the player's own column through
  `weather::precipitation_parts`'s rain-shadow branch, so crossing that contour steps the
  haze and the sun (diagnosed at C-163, the repair left to the user); the world going black
  with only the shell is reported and not reproduced (C-163).
* **H-7.10** — what the save the view marks — remains the one open question on the
  record (C-77).

## 5 · Evidence index

* **`logs/`** — raw runs, by era: `c154-*` workspace tests; `c158-*` sky/exposure;
  `c159-*` (water GPU twice, biome censuses, tick-over-world twice, HUD legibility
  twice, mount handover, worked examples, client GPU tests, workspace tests);
  `c160-*` (sub-biome censuses, tick-over-world, per-crate suites); `c163d-*` (the GPU
  pattern stages, model/appearance/shapers/client/package suites, both builds);
  `health-test-run-port-*` (shell port, per module); `board-report*.txt`,
  `laya-report.txt`, `laya-decisions/`, `agent-inbox/` (recipes the game wrote),
  `structurizr*` (architecture renders), `audit-*.log`, `biome-*.log`.
* **Worked examples** (run with `cargo run -p voxel-shapers --example NAME`):
  `biome-cost` (202 → 5.6 µs), `surface-census` (the map's letters,
  habitat-modulated at C-160), `habitat-census` (the sub-biome map, C-160), `flora-census`
  (tree counts + a slice through a real conifer), `tick-over-world` (0/5445),
  `biome-drift`, `relief-profile`, `water-ledger`, `stage-clocks`; client:
  `surface-colours`.
* **Commands that define "green"** (per crate, unfiltered, per C-45):
  `cargo test --offline -p voxel-shapers|voxel-stream|voxel-client|voxel-work|voxel-appearance`;
  `cargo build --offline -p shploingus`; explicit GPU runs with `--ignored`.

*Coverage: every entry C-1…C-165 of `docs/LEDGER.md` is represented above; the
detailed record and its subsections remain the authority. Generated 2026-10-05.*
