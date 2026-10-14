//! **The registry of unified systems.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2, which itself extends
//! `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md`.
//!
//! §2 closed the list at eight and reserved a ninth for a decision on the record. **That decision was
//! taken on 2026-10-12 and is recorded in §16**, which admits the five systems the mission's
//! completeness clause names as **U9–U13**. The table below carries all thirteen, so what is built and
//! what is only admitted are both visible values rather than something a reader has to know.
//!
//! The registry is not documentation beside the code — it is the list the binary prints and the list
//! the tests hold to its own totals (P6: a file exists for a stated purpose, and this purpose is to
//! make the program's scope a value a test can read).

/// One unified system: what it binds, its witness, its fresh-code boundary, its interface, its slice.
pub struct UnifiedSystem {
    /// `U1` … `U13`.
    pub id: &'static str,
    pub name: &'static str,
    /// The honest state: `specified`, `in progress`, or `absent`. A system is never `implied`.
    pub status: &'static str,
    /// What the system binds into one whole.
    pub unifies: &'static str,
    /// The legacy paths whose **behaviour** defines the contract (evidence only).
    pub witness: &'static str,
    /// What the clean room **may re-derive from** (documents/papers/declared formats).
    pub derived_from: &'static str,
    /// **The bytes a ported seam of this system reads, pinned by path + sha256** (Constitution P7 as
    /// amended to v2.0.0, 2026-10-13). This field replaces the retired `must_not_copy`: under the amended
    /// principle a port owes a **pin**, not a prohibition, and a row with no ported seam says so in its own
    /// words rather than borrowing the old one.
    pub pinned_by: &'static str,
    /// The surface the system must expose to its callers.
    pub interface: &'static str,
    /// The order in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5.
    pub slice: u8,
}

/// The thirteen unified systems, in slice order: §2's eight, then §16's five.
pub const SYSTEMS: [UnifiedSystem; 13] = [
    UnifiedSystem {
        id: "U1",
        name: "Presentation",
        status: "specified + one measured behaviour; **the visible client built (2026-10-13, §36)**: \
                 `src/surface.rs` owns the client's pixels and their presentation — a frame drawn from the \
                 world the tree already builds through the contract's own one `shade` call, presented in a \
                 **real window** built against the platform's own interface (no crate is added: the record's \
                 dependency rule forbids one), one tick of the plane's own law per frame, with the window's \
                 own pixels read back and compared. **The window now draws the world's own geometry, and each \
                 reading is named with one owner (2026-10-13, §41, `LEDGER.md` C-193)**: the **drawn \
                 reading** — `src/drawn.rs`, the mesh's own four corners and two triangles per rectangle, one \
                 `shade` call per rectangle, one camera with a depth buffer — is what the window presents, \
                 its subject derived through `surface::region_mesh`, and the **plan reading** \
                 (`Surface::render`) is **kept and named**: it is the evidence run's own frame, the one the \
                 `U1 client:` lines measure, so no frame is drawn by two readings and neither is \
                 unlabelled. **And a person can drive it (2026-10-14e)**: `src/viewpoint.rs` owns the one camera \
                 state (`Viewpoint` — the eye and a *stored* unit look, so a frame can come back byte for byte) \
                 and the one intent vocabulary (`Wish`: moved, turned and tilted as held intents, never as \
                 keys), `src/controller.rs` owns the session built on them (`Driven`: the law's own state, the \
                 streamer's own resident set, the person's own eye, and what each step cost), \
                 `surface::step_driven` is **the one door a step comes through** — the eye moves, the anchor is \
                 read off the eye's own chunk, the residency is observed there and the frame is drawn from \
                 there — and the two translations into a `Wish` are the window's own keys (`W`/`S`, `D`/`A`, \
                 `Space`/`Control`, the four arrows, the parent's own free-flight map) and the binary's new \
                 `drive` mode (one line of intent and a count of steps). Measured: eight step(s) west move the \
                 eye 4.80 m, move the person's own chunk from (1, 1, 2) to (0, 1, 2) and change every frame's \
                 own digest, and eight step(s) east put the eye back **bit for bit** and draw the declared \
                 view's own frame again — 1333 rectangle(s), 147 808 of 331 776 pixel(s), digest `7816bf6a…`; \
                 the drawn reading's own bars moved with the camera (the frustum bar became *the frame is drawn \
                 from the eye the person stands at*, and *the rectangles the eye does not see are counted: \
                 `clipped == 2 × unseen`*, because a person stands **inside** the world they draw). \
                 Owed and named: the **GPU pass** (T009's other half — a \
                 shader checked against `presentation::shade`), a resizable window, the Sun's azimuth \
                 (§24's own owed item) — and, with the camera now driven: a **body** (this is a free-flying eye; \
                 the parent's own body has no row here), a **pointer** (a turn is a key, not a device motion), a \
                 **frame-rate-independent step**, and a **world wider than its own horizon** (this world's 4 \
                 chunk(s) span 45.2548 m inside the declared 256 m, so no camera in it can make a chunk leave \
                 the residency)",
        unifies: "OpenPBR-input derivation, Source 2 stylisation, the GPU pipeline, sky, shell painting",
        witness: "crates/voxel-appearance, crates/voxel-client (+ 7 .wgsl)",
        derived_from: "docs/APPEARANCE.md §1, docs/DESIGN-SPEC.md §2 F13, the FMA bar, LEDGER C-39",
        pinned_by: "the port's own pin — docs/baselines/port-appearance-seam/pinned-parent-bytes.txt: \
                    crates/voxel-appearance (11 .rs) + crates/voxel-client (49 .rs) + docs/APPEARANCE.md \
                    + Cargo.lock, 71 paths by sha256, verified 2026-10-13 (`sha256sum -c`, exit 0); the \
                    port is the `voxel-appearance` `path` dependency in clean-room/Cargo.toml and its \
                    gate is `tests/u1_port_flicker.rs`; **and the controller's own read-set** (2026-10-14e, \
                    §12 of the same file): the parent's input and camera bytes this seam was read from — \
                    `crates/voxel-client/src/motion.rs`, `crates/voxel-client/src/camera.rs` and \
                    `crates/shploingus/src/main.rs` — pinned by path + sha256, with the crate-level blocker \
                    that keeps them a **source rather than a dependency** quoted in \
                    `docs/baselines/port-appearance-seam/parent-blocker-2026-10-14e.txt`",
        interface: "shade(surface, normal, light, view) -> Rgb; band_count; Renderer seam; surface::{Surface::render → the plan reading's pixels, present_world → a real window showing the drawn reading from the person's own eye, measured_frame → the frame's own figures, region_mesh, draw_plane, draw_from, step_driven, residency_at, residency_of, home_viewpoint, home_anchor}; viewpoint::{Viewpoint, Wish, Step, chunk_over_plan}; controller::{Driven, check_u1_controller}; drawn::{draw, Camera, Drawn}; and the binary's `window` and `drive` modes",
        slice: 1,
    },
    UnifiedSystem {
        id: "U2",
        name: "World model and its vocabulary",
        status: "in progress (world, shapes, chain; audit findings fixed; **the cubic world's own cells and \
                 their one identity, and the column world as a reading of them (2026-10-13, §37)**: \
                 `src/chunk.rs` owns the declared 32³ chunks over `coordinates`' own algebra, one digest over \
                 every cell, and one reading (`CubicWorld::column`) that `being::WorldRead::over_cubic` hands \
                 to the existing consumers unchanged; the **column** representation is refused **for cause** \
                 (§37, `LEDGER.md` C-189) and `src/surface.rs` is kept as a **plan-view instrument**, so the \
                 owed GPU pass is checked against that instrument's own projection and not against a shader \
                 for a picture whose shape was refused; and the **water law now steps those cells** (2026-10-13, \
                 §38, `LEDGER.md` C-190): the law that was on the generator-fed column field beside them holds \
                 the world's own cells as its one state, so the plane a being's need is met through *is* the \
                 world the automaton stepped, and the quantised reading's routing difference is measured and \
                 answered (2 033 of 2 304 columns, every one a voxel effect, the reading the declared voxel at \
                 all 2 304) rather than left open; axis vocabulary owed)",
        unifies: "coordinates, the cubic chunk's own cells, the 4x4x4 lattice and 256x level, matter, materials, shapes, chain, tickets",
        witness: "crates/voxel-model",
        derived_from: "docs/DESIGN-SPEC.md §1-§5, docs/SHAPERS.md §2, docs/MODS-MATERIALS.md, the Axis/Laya rules",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "Coord; Subvoxel; MaterialRegistry; RefusalSet; ShaperSet; Profile; Chain; chunk::{CubicWorld, Cell, CubicColumn}",
        slice: 2,
    },
    UnifiedSystem {
        id: "U3",
        name: "World generation (the shapers)",
        status: "in progress (15-stage roster declared and validated; 13 stage bodies landed; one \
                 digest at 1/2/4/8 workers; 2 stages refuse by name by decision — caves, whose \
                 dissolution field is not declared, and earthworks, the record and not a generator; \
                 delta's avulsion state is owed inside a landed stage)",
        unifies: "noise primitives and the fifteen shaper stages plus the cadence runtime",
        witness: "crates/voxel-noise, crates/voxel-shapers",
        derived_from: "docs/SHAPERS.md, docs/MISSING-SYSTEMS.md, docs/DESIGN-SPEC.md §2 F4-F10",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "Declared; Stage; ShaperSet; ProcessScheduler; TickDriver; field_at",
        slice: 3,
    },
    UnifiedSystem {
        id: "U4",
        name: "Meshing, streaming and work",
        status: "in progress (field-derived mesh with greedy merge; level ladder 0..=2; four typed \
                 horizons; resident set keyed by world truth with exact invalidation; work ledger \
                 counted where the work happens; budget by name; one digest at 1/2/4/8 workers; **and the \
                 mesh's own rectangles reaching pixels (2026-10-13, §41)**: `src/drawn.rs` is the drawn \
                 reading — the mesh's four corners and two triangles per rectangle, one `shade` call per \
                 rectangle, one camera with a depth buffer — and the window's own frame is that reading, \
                 with the counts, the coverage, the depth extents and the state digest the frame came from \
                 printed by the run; **and the mesh seam bridged and measured (2026-10-13; its structure split into the `src/port/` directory on 2026-10-14 — `port/adapter.rs` the seam, `port/differential.rs` the measurement)**: \
                 `src/port/` drives the parent's own `voxel_mesh::Mesher` with this crate's own cells \
                 through the parent's `VoxelSource`/`ChunkNeighborhood` traits, and every divergence from this \
                 crate's own derivation is accounted for by a named reason — 0 unnamed over 8 cases, 16 558 \
                 unit face(s) the world owns compared (capture: `docs/baselines/port-appearance-seam/\
                 port-mesh-2026-10-13.txt`); the parent's **streaming is not ported** and the reason is its own \
                 signature (`voxel_stream::Resident` takes a concrete `WorldSource`, i.e. the parent's \
                 generator), while the mesh itself is still this crate's own derivation because the two \
                 derivations are not equal — the parent draws the world's own edge as air and this crate draws \
                 it as an edge; **and the streamed residency that consumes that keyed door (2026-10-14c)**: \
                 `src/stream.rs` keeps the chunks around a camera's own chunk while the declared residency \
                 horizon keeps them, derives each one **on the spot** through the door, serves a resident chunk \
                 with no derivation at all, evicts what the horizon releases and re-derives it when the camera \
                 comes back — 4 of this world's 4 chunk(s) held at each of the parent's four horizons (which \
                 name 4 913 / 15 625 / 117 649 / 7 189 057 chunk(s) and generate every one of them), the first \
                 eviction at 45.2548 m, the world's own far corner; **and the residency in the drawn frame \
                 (2026-10-14d)**: the residency now streams **the state the client shows** and the drawn \
                 door takes **the residency itself** — `surface::{client_state, residency_of, region_mesh, \
                 draw_plane}` — so the held cells are the world's own store, one `CubicWorld` and no second \
                 collection, and a frame's mesh state **is** the residency's own identity (measured: 4 of 4 \
                 chunk(s) / 131 072 cell(s) held, 0 differing of 131 072 against the shown world's own door, \
                 while against the declared surface the fields make 3 673 of 131 072 cell(s) differ — the law's \
                 own 200 tick(s), the number the port measured, reached from the residency's own side; the \
                 drawn reading reproduces §42's frame byte for byte — 1333 quads, 147808 of 331776 pixel(s), \
                 frame digest `7816bf6a…` — and the window's own client area is the frame's own bytes at \
                 66 355 200 of 66 355 200 pixel(s); a residency holding a part of the region is refused by the \
                 region's own name, so no frame is a picture of less than the region); the displaced 256x \
                 level, a region's own \
                 residency rather than its one-footprint stitch, a strided field request and a ceiling on what \
                 may be resident at once are owed)",
        unifies: "sparse voxel octree, greedy mesh, 256x refinement, four horizons, resident set, pool, budget",
        witness: "crates/voxel-mesh, crates/voxel-stream, crates/voxel-work",
        derived_from: "docs/DESIGN-SPEC.md §4 SPEC-3/SPEC-5, LEDGER C-43, C-46",
        pinned_by: "the mesh port's own pin, appended rather than rewritten — \
                    docs/baselines/port-appearance-seam/pinned-parent-bytes.txt lines 72..87 (16 paths under \
                    crates/voxel-mesh, crates/voxel-model and crates/voxel-stream; `sha256sum -c` 87 of 87 OK). \
                    The bytes the ported call reads are `voxel-mesh` (the mesher, its two traits and the \
                    octree) and `voxel-model` (the types they are spoken in); the `voxel-stream` bytes are \
                    pinned because this pass read them to establish that the residency is **not** portable — \
                    `Resident::new(source: WorldSource)` is concrete, so only the parent's own generator can \
                    be streamed (U1's row is the worked example of a seam that is ported)",
        interface: "Mesh; Octree; refine; WorldSource; Horizon; Resident; Pool; Budget; TickReceipt; \
                    drawn::{draw, Camera, Drawn}, surface::{client_state, residency_of, region_mesh, \
                    draw_plane, drawn_frame}",
        slice: 4,
    },
    UnifiedSystem {
        id: "U5",
        name: "Instrumentation, light and audio",
        status: "built (2026-10-12, §24; the voices landed §30): the one sensor concept with the wall \
                 and the overlay as its consumers, a min/max band per item, a weighing gate whose \
                 answer is a mass and nothing else, the virtual gauge wall in one controller; one band \
                 table and the closed-form solver (sky, beam, twilight, the Moon as a reflectance, \
                 two-droplet geometries); the feeling wheel read as a value, and the voices — one \
                 bounded buffer out of the wheel's own signature, the weather's own gain and the \
                 field's own noise, with an absent, undeclared, uncalibrated or contradictory reading \
                 refused by name — and the onset layer's own read for the audio seam. Owed and named: \
                 ozone, an azimuth, the per-cell frame format, the material's own voice and wind, and \
                 the music layer",
        unifies: "the sensor concept, min/max bands, the weighing gate, the gauge wall, the light solver, audio",
        witness: "crates/voxel-instrument, crates/voxel-light, crates/voxel-audio",
        derived_from: "docs/DESIGN-SPEC.md §3 S-03/S-12/S-13, FC-2.16, docs/light-solver.md, docs/lighting-sky-and-moon.md, the feeling-wheel vocabulary (its own SKILL.md and reference.md)",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "Sensor; Band; GaugeReading; GaugeWall; Crossing; Mass (and nothing else); Solver; \
                    Spectra; the audio seam (onset, gain); Feeling; voice -> Voice (samples, levels, \
                    signature, digest)",
        slice: 5,
    },
    UnifiedSystem {
        id: "U6",
        name: "The record and the API layer",
        status: "in progress (the gate: a closed eight-row table, the canonical request line, the \
                 answer and its claims, the checks, the effect, the receipt, and the parse/world \
                 refusal split with the world's refusals recorded through the chain's one writer; \
                 the stdio transport: one line in, one response record out, served by the binary's \
                 `serve` mode and driven as a separate process by the binary's own evidence and the \
                 tests; the content gate: a declared row consumed **in place by reference** — as \
                 `intake`'s own seam and as the table's `admit` row, which arrives over the wire by \
                 reference (`path + digest`, never bytes) and answers what the row became — judged \
                 on **seven** named axes, `admit`/`inspect` -> `Established` and never a bool, with \
                 an unchecked axis never a pass and a pin still not a claim a request line can carry; \
                 the last axis is **conformance against the world's own declarations** — a row may \
                 require stages of U3's `Declared` and materials of U2's `MaterialRegistry` (the \
                 table's two **optional** arguments, `requires_stages`/`requires_materials`, whose \
                 absence is the declaration *this row requires nothing*), refused `stage_undeclared` \
                 with the stage named and `unadmitted_material` in the registry's own words, with a \
                 requirement that is **owed** or names nothing reachable through the intake's seam \
                 but not yet spellable by a served line, and with neither ever counted as a pass; \
                 `save`/`load` arrived as the seventh and eighth rows of the table (2026-10-12, §28) \
                 by reference, so the table is closed at eight and U9's document form reads through \
                 this gate; and the **policy chokepoint** landed (2026-10-12, §29) — `policy.rs`'s one \
                 decision function, every served row consulting it at one call site, with deny by \
                 default made structural (a closed action vocabulary and an exhaustive matrix with no \
                 wildcard arm), grants refused construction without a granter or a scope, a refusal that \
                 quotes the list, and every denial **recorded** on the one chain; owed: the gate's \
                 remainder — the document record and its compression, the structural judge, the ballot, \
                 the recipe, the API document, the policy matrix's **remainder** (the self-describing \
                 protocol and the tool-surface narrowing over a mod-supplied tool list, neither of \
                 which has a caller in this build) — and the legacy-vocabulary bijection)",
        unifies: "intake gate, document record, structural judge, ballot, census, policy, recipe, apidoc, bridge",
        witness: "crates/voxel-package (+ crates/voxel-mcp as transport)",
        derived_from: "docs/DESIGN-SPEC.md §2 F1-F2, docs/UI-SPEC.md, docs/SHELL-PORT.md, docs/MODS-MATERIALS.md",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "intake::{Source, Claim, Row, Requirement, Requirements, Axis, Establishment, Finding, WorldDeclarations, Intake::over/admit/inspect -> Established, Intake::census}; Compendium::derive; StructuralJudge; Ballot; recipe; apidoc::render; the policy chokepoint: policy::{Action, Scope, Grant, Caller, PermissionClass, permission_of, decide, check_matrix} consulted by the gate at one call site; bridge::TOOLS + Tool::action + Receipt; reference::{gate, caller, serve_stdio}; transport::serve",
        slice: 6,
    },
    UnifiedSystem {
        id: "U7",
        name: "The decision layer (axis, laya, von)",
        status: "built (2026-10-12, §25): the seven ladders — three declared from documents, four \
                 **owed by name** — with the two prohibitions and round 5's bare-token rule, the coverage \
                 table's total pin and its anti-fabrication guard; the closed three-word verdict, the \
                 typed question, the ballot whose floor is declared before the answer exists, the \
                 structural-judge seam and the four fail-closed rules, each refused by its own name; and \
                 the classifier — the record's own declared facts (twelve features over the format's words) \
                 with its claim's fourteen features beside them, calibrated over the sixteen labelled \
                 records at five life stages — answering a ballot with `Origin::Oracle` while its \
                 leave-one-out accuracy beats the majority class (0.5000 against 0.3125, margin +0.1875, \
                 with the claim-only reading 0.0000 printed beside it because that is the finding that \
                 moved the question to the head). Owed and named: the four ladders' token lists (U12's \
                 vocabulary), the score instrument's rungs, the `noul`/`score` calibration sets (a `noul` \
                 question is **refused by name** until its set exists), and a \
                 model-backed instrument (this build's `von` is structural, on CPU, with no dependency \
                 added)",
        unifies: "the seven axes, the Laya questions, the ballot, the calibrated oracle, the von classifier",
        witness: "crates/voxel-model/src/{axis,laya}.rs, crates/voxel-package/src/{oracle,ballot}.rs, crates/voxel-von",
        derived_from: "the Axis rules in LEDGER (rule 51-55, round-5 bare-token), the plan's Lane C join, patches/PATCH-023 (the ballot), patches/PATCH-039 (the five stages, and that the stage question is answerable from heads)",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "AxisKind + Ladder + refuse_interchange/refuse_reduction/refuse_bare + COVERAGE/check_coverage; Question{Choice,Score,Noul}; Verdict{Belongs,NotBelongs,NoReading}; Ballot; Measurement (four accuracies and the majority class); StructuralJudge + Unreachable; laya::answer and the four fail-closed rules; von::Classifier (Declared/Feature, calibrate, classified, carriers) and von::claimed_vocabulary",
        // The registry's `slice` is this table's **position** (the test asserts `slice == i + 1`), while the
        // plan's §2 `slice:` fields name the §5 *row* — where row 5 carries U5 and U6 together, so the two
        // readings differ for U6/U7/U8 by §16.3's own correction. Named here because the difference is the
        // sort of thing a reader would otherwise take for a bug.
        slice: 7,
    },
    UnifiedSystem {
        id: "U8",
        name: "The game binary and the shell UI",
        status: "built (2026-10-12, §27): the only window — a Mount-shaped pane tree driven by a declared \
                 Recipe (six declared windows, and the mount adds none it was not handed, so there is no \
                 second registry), the shell/HUD layer toggle over one window set, the one \
                 pointer-ownership decision read once and passed by value into both the drawing and the \
                 grab so the glyph and the grab cannot disagree, the four-bucket admitted constants table \
                 (16 rows; an unadmitted name is refused by name) and a visibly drawn 100 × 30 frame \
                 whose panes read the systems already built (the registry, the reference world, its \
                 materials, the stage roster, a derived chunk's geometry and the light table). The `shell` \
                 mode is the interactive demonstration. Owed and named: undock, the splitter-drag resize, \
                 the console's five graded levels, the toast's final form, and the gholtzap/lintapple **P8** \
                 gate, which U13 owns and this slice does not take",
        unifies: "the only window, the mount window manager, the design system, input ownership, the frame loop",
        witness: "crates/shploingus, crates/voxel-ui, crates/voxel-client/src/shell",
        derived_from: "docs/SHELL-PORT.md, docs/UI-SPEC.md, docs/DESIGN-SPEC.md §3, portal_simulator.py's structure",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "shell::Mount over a declared shell::Recipe (split/close/focus/toggle_layer/read_pointer/layout), the shell::Content readers, the four-bucket shell::CONSTANTS table, and shell::frame → a drawn Canvas; plus the binary's `shell` mode",
        slice: 8,
    },
    // ---- §16: the mission's five, admitted as U9–U13 (2026-10-12) ----
    UnifiedSystem {
        id: "U9",
        name: "Saves and loads, and the world's own seed systems",
        status: "in progress (the written form of a world: the exact length-framed encoding and its \
                 trailer; five declared per-domain stream seeds carried and checked in both \
                 directions; the ruleset identity; the record carried in its own wire form and \
                 declared by head and length; six named refusals; a real file round trip and one \
                 generation digest at 1/2/4/8 workers; and the **document form** — a world written at \
                 and read back from a declared reference (`path + digest`) through **U6's content \
                 gate**, the one place that gate is a hard prerequisite, carried as the table's own \
                 `save`/`load` rows and as `Save::read_document`. Owed: a save of a resident set, \
                 compression, and a versioned migration path)",
        unifies: "the save, the load, and the seed systems of a world",
        witness: "crates/voxel-client's `World` (seed + tick + config), crates/voxel-shapers' `WorldSpec` with its per-stage seed derivation",
        derived_from: "docs/DESIGN-SPEC.md §1.3 and §2 F1, SPEC-3; LEDGER C-96.1 and H-7.20; docs/CIVILIAN-PORT.md (GeneratorIdentity); docs/GAIA.md §5c/§11",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "Save::of / Save::parse / Save::load -> Loaded; SaveIdentity; StreamSeed; Ruleset; LoadReport; and the document form: Document::write / Save::read_document, which reads only through U6's `Intake::read`",
        slice: 9,
    },
    UnifiedSystem {
        id: "U10",
        name: "The gameplay tree the mods references imply",
        status: "in progress (the tree: **one entry per reference** in `docs/MODS-MAP.md` §1–§5, in the \
                 record's own order, each carrying the record's own four-word disposition — the count \
                 this build derives is printed beside the record's own figures, un-averaged — and **one \
                 row per technology** under the double-cover rule: 43 rows, each U6's own row, whose \
                 document is read **in place** and admitted through the gate the way an `admit` row is, \
                 so a document that is absent refuses `source_absent` and one whose bytes have moved \
                 refuses `source_digest_mismatch`, a refused reference refuses `licence_refused` by the \
                 owner's own name, and an owed reference is named and never admitted. Owed, and printed \
                 as owed: the recipe document's row-kind key and version, the duplicate-row refusal's \
                 name, a verb that serves the whole tree, and the per-row family assignment)",
        unifies: "the ordered set of playable systems the mods references imply, as one declared tree of rows",
        witness: "docs/MODS-MAP.md (**106 references** — 10 §1 + 42 §2 + 19 §3 + 35 §4, one row per reference with its clean-repo destination; its source file's 91 URLs and this repo's copy's 97 are counted in the header), docs/MODS-MATERIALS.md",
        derived_from: "docs/DESIGN-SPEC.md ruling 3 and §3's S-01..S-24 slots; docs/MODS-MAP.md §1-§5; docs/SHAPERS.md",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "gameplay::Tree::of_record/entries/rows/first/declared/declare + Disposition + Entry + Technology; a tree of declared rows: feature, source document (path + digest), licence verdict, the system that carries it, its own acceptance check",
        slice: 10,
    },
    UnifiedSystem {
        id: "U11",
        name: "planet shploingus (the planet-scale world)",
        status: "absent (admitted by §16; waits on U4's octree, refine and streamed residency policy, \
                 and on a declared scale/extent ruling — the 20-level ladder's 16 777.216 km is short \
                 of an Earth-sized circumference)",
        unifies: "the planet as one world: its scale contract, its addressing, and the residency policy that lets it exist without being resident",
        witness: "nothing in crates/ implements a planet; the outside tree's F:\\planet-shploingus\\docs\\ART-DIRECTION.md is reference, never authority",
        derived_from: "docs/DESIGN-SPEC.md §5 and SPEC-5; docs/SHAPERS.md §2 (tectonics: Kreischer & Bommes 2019); docs/PLAN-2026-10-08-cloud-cubes.md",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "a declared scale/extent contract; U4's Octree + refine + Resident's streamed policy",
        slice: 11,
    },
    UnifiedSystem {
        id: "U12",
        name: "Civilian organisations for mobs and autonomous players",
        status: "**first slice built (2026-10-13, §32)**: **the first mob** (`src/being.rs`) — one \
                 organisation with a declared charter, an office and the **floor** its charter declares, and \
                 a being whose need is a fold over its own records (no need tick), whose world is the game's \
                 own cells — the cubic world the declared fields are filled into, read through the one \
                 reading every consumer shares and walked along `d8_receivers` — whose choice is a **laya \
                 question read by von**, and whose every act and refusal is a governed event on the **one** \
                 `Chain`. Row 12's check is measured: a granted act is taken, an act the office does not reach \
                 (`save_world`) is refused `policy_denied` and **recorded**, a need the world cannot meet \
                 where the being may reach is refused `water_unreachable`, an act below its charter's floor \
                 is refused `stage_below_floor`, and an unreadable being is refused `no_reading`. **The plane \
                 steps, on the cells (2026-10-13, §34 restated §38)**: `src/automaton.rs` is the game's own \
                 routing as a cellular automaton over the cubic world's own cells — one law, one cell state, \
                 one synchronous tick, no second needs model, scheduler or automaton path — so the being's \
                 need is met through the game's own automata **over time** rather than by one static \
                 evaluation at Tick(0), and `WorldRead::over_cubic` is a reading of the world the law actually \
                 stepped. The route difference §37 left open is measured and printed: 2 033 of 2 304 columns \
                 route differently from the generator's float surface, every one of them a voxel effect \
                 (1 970 swallowed whole inside their own voxel, 63 tied at the steepest quantised drop), none \
                 unexplained, and the reading is the declared voxel at all 2 304 columns — **a faithful \
                 consequence of the quantisation, not a defect**. Still owed by name: the organisation's own \
                 vocabulary (U2's T016 — the office rank and the certification bands, printed owed in \
                 `axis.rs`), U6's self-describing protocol, the other kinds of being (plants, incarnate \
                 pilots), relations and the court, the picture of a population (many beings, one world), and a \
                 being-act declared-fact vocabulary (an act that rests on a declared fact rather than on a \
                 draft's own reading)",
        unifies: "the organisations civilians, mobs and autonomous players belong to and act through: membership, authority, permitted verbs, and the record of what was done",
        witness: "crates/voxel-model/src/civilian (BLD-0221, whose `pub mod civilian;` is absent from that crate's lib.rs), the government vocabulary in LEDGER, docs/GAIA.md for an autonomous population",
        derived_from: "the Axis/Laya rules quoted in LEDGER; docs/GAIA.md §2-§9; docs/CIVILIAN-PORT.md as a document; DESIGN-SPEC §2 F11 and F8",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "Organisation with declared membership and authority; an actor that asks the one tool table what it may do; every act a governed event on the one Chain",
        slice: 12,
    },
    UnifiedSystem {
        id: "U13",
        name: "The shell completed, and the gholtzap/lintapple gate",
        status: "absent (admitted by §16; waits on U1's remainder, U6's admitted constants table, U8, \
                 and the P8 gate, which its own acceptance check closes)",
        unifies: "the mission's more complete shell as an acceptance surface: U1's shade and sky, admitted constants, and its own conformance gate",
        witness: "crates/shploingus, crates/voxel-ui, crates/voxel-client/src/shell",
        derived_from: "docs/SHELL-PORT.md (the four-bucket rule), docs/UI-SPEC.md, docs/DESIGN-SPEC.md §3, portal_simulator.py's structure, the linter's own published rules",
        pinned_by: "none yet — no seam of this system is ported: the witness above stays a witness, not a \
                    source (U1's row is the worked example)",
        interface: "U8's Mount pane tree plus the gate: the shell's drawn values and tool surface checked by P8, with the result recorded",
        slice: 13,
    },
];

/// How many unified systems this build knows. Written down, and pinned to [`SYSTEMS`]' length.
pub const SYSTEM_COUNT: usize = 13;

/// The registry as printed text. One block per system; nothing summarised away.
pub fn render() -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "The clean room's {} unified systems (docs/CLEAN-ROOM-PLAN-2026-10-11.md §2, extended by §16):\n\n",
        SYSTEM_COUNT
    ));
    for s in &SYSTEMS {
        out.push_str(&format!("[{}] {}  (slice {}) — {}\n", s.id, s.name, s.slice, s.status));
        out.push_str(&format!("     unifies:       {}\n", s.unifies));
        out.push_str(&format!("     witness:       {}\n", s.witness));
        out.push_str(&format!("     derived from:  {}\n", s.derived_from));
        out.push_str(&format!("     pinned by:     {}\n", s.pinned_by));
        out.push_str(&format!("     interface:     {}\n", s.interface));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_is_closed_and_ordered() {
        assert_eq!(SYSTEMS.len(), SYSTEM_COUNT, "the count and the table must agree");

        // Unique ids.
        for (i, s) in SYSTEMS.iter().enumerate() {
            assert!(
                SYSTEMS[..i].iter().all(|other| other.id != s.id),
                "duplicate id {}",
                s.id
            );
        }

        // Ids are U1..U8 in order, and slices ascend.
        for (i, s) in SYSTEMS.iter().enumerate() {
            assert_eq!(s.id, format!("U{}", i + 1), "ids must be contiguous and ordered");
            assert_eq!(s.slice as usize, i + 1, "slice order must match table order");
        }

        // Every system names a witness, a fresh boundary and an interface, or it is not a system.
        for s in &SYSTEMS {
            assert!(!s.status.is_empty(), "{} has no status", s.id);
            assert!(!s.witness.is_empty(), "{} has no witness", s.id);
            assert!(!s.derived_from.is_empty(), "{} has no fresh boundary", s.id);
            assert!(
                !s.pinned_by.is_empty(),
                "{} names nothing it is pinned by (and no reason it is not yet ported)",
                s.id
            );
            assert!(!s.interface.is_empty(), "{} exposes no interface", s.id);
        }
    }

    #[test]
    fn the_registry_prints_every_system() {
        let text = render();
        for s in &SYSTEMS {
            assert!(text.contains(s.id), "the printout omits {}", s.id);
            assert!(text.contains(s.name), "the printout omits {}'s name", s.id);
        }
    }
}
