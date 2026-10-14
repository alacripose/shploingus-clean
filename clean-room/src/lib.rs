//! **The clean-room library.**
//!
//! New code beside the legacy tree (Constitution P7). Each module names, in its own header, the
//! document it was derived from. The modules are the unified systems' fresh implementations; the
//! binary in `main.rs` is a thin consumer, so the API this crate exposes is the same surface its
//! tests and its running binary exercise.
//!
//! Built systems: **U1 Presentation** (`presentation`), **U2 World model** (`coordinates`,
//! `chunk` — the cubic world's own cells, their one identity, and the column world as a *reading* of
//! them — `material`, `world`, `profile`, `shaper`, `chain`, `refusal`, `digest`), **U3 World generation**
//! (`stage`, `process`, `shapers`, `noise`, `field`), **U4 Geometry** (`mesh`: field-derived
//! meshes, the level ladder, the four horizons, the resident set and the work ledger; `stream`: the
//! **streamed residency** — the chunks a camera keeps, each derived on demand through the world's own
//! keyed door and released when the horizon no longer keeps it) and
//! **U6 The record and the API layer** (`bridge`: the closed tool table, the request form, the
//! receipt and its claims, and the gate that dispatches to the owners' seams — one public surface
//! over five files, `bridge::table`, `bridge::request`, `bridge::receipt`, `bridge::wire` and
//! `bridge::gate`; `transport`: the declared line protocol that serves that table over stdio, one
//! line in and one response record out; `reference`: the one reference world a session's two sides
//! compare answers through; `intake`: the content gate's first slice — a declared row **consumed in
//! place by reference**, judged on named axes, admitted as `Established` (never a bool), with the
//! census that never passes an unchecked axis; `policy`: the policy chokepoint — one decision
//! function every served row passes through at one call site, a closed action vocabulary with an
//! exhaustive class matrix that has no wildcard arm, grants refused construction without a granter or
//! a scope, a denial that quotes the list, and every denial recorded) and **U9 Saves and loads** (`save`: the written form of a world — its
//! identity, its five declared sub-stream seeds, its ruleset identity and its one record — with the
//! parser and the correspondence checks that make a load exact or refused). **U10 The gameplay tree**
//! (`gameplay`: one entry per reference in `docs/MODS-MAP.md` §1–§5, the record's own four-word
//! disposition for each, and one row per technology under the double-cover rule — a list of U6's own
//! rows, with the four parts the record has not decided printed as **owed**). **U5 Instrumentation,
//! light and audio** (`measure`: the one sensor concept, a per-item min/max band, the weighing gate
//! whose answer is a mass and nothing else, and the gauge wall — with a quantity that cannot be
//! weighed refused by name; `light`: one band table and the closed-form solver that reads it — sky,
//! beam, twilight, the Moon as a reflectance, droplet optics; `feeling`: the feeling wheel read as a
//! value — six families anchored at real wavelengths, the radius the intensity and a signed drift that
//! walks the peak, with an absent, undeclared, uncalibrated or contradictory reading refused by name;
//! `audio`: the onset layer's own read, so an audio layer can ask *is it raining right now*, and the
//! **voices** — one bounded buffer out of the wheel's own signature, the weather's own gain and the
//! field's own noise, with its levels and its digest printed beside it). **U7 The decision layer** (`axis`: the seven
//! ladders and the three ways reading one is a bug — the two prohibitions, the coverage table's total
//! pin and its anti-fabrication guard — with four ladders **owed by name** where no document in this
//! tree declares their tokens; `laya`: the closed three-word verdict, the typed question, the ballot
//! whose floor is declared before the answer exists, the structural-judge seam and the four fail-closed
//! rules; `von`: the classifier — the record's **head's** declared facts, with its claim's fourteen
//! features read beside them by **one** fence reader, a calibration over the sixteen labelled records the
//! plan names, its leave-one-out accuracy read against the majority class, an answer whose origin is the
//! proof's to carry rather than the claimant's to declare, and a **`noul` question refused by name**
//! because no set calibrates it). **U12 Civilians** (`being`: the first mob — one organisation with
//! declared membership and authority, a being whose need is a fold over its own records, whose choice is
//! a laya question read by von, and whose every act is a governed event on the one chain; `automaton`:
//! the cubic world's own cells' **stepping law** (§38) — one water depth per cell, one synchronous tick of
//! the game's own routing over the cells' own surfaces with two declared constants, and no schedule of its
//! own, so a rained-on or drained column changes where the being's walk reaches water). **The client**
//! (`surface`: two **named** readings of one world — a world's own columns shaded through the contract's one
//! `shade` call, and the mesher's **own rectangles** drawn under one camera with depth — and **one real
//! window** the windowed one is presented in, built against the platform's own window interface because the
//! record's dependency rule forbids a windowing crate and this client's own ruling says so out loud; `drawn`:
//! the drawn reading itself — the mesh's vertices and indices to pixels, the one camera, the depth buffer, and
//! the counts that say what reached them; `viewpoint`: **the person's own viewpoint** — one state for
//! *where the eye stands and which way it looks* and one intent vocabulary (`Wish`, in intent rather than in
//! keys), with the drawn camera **derived** from it and the chunk it stands over the anchor a residency is
//! observed at; `body`: **the client's own body** (2026-10-14g) — the feet, the velocity and the two
//! traversals, with one walking law (a jump, a horizontal move blocked by the world's own cells and climbed by
//! a step, a vertical move stopped by the floor, gravity last) and **flight kept as an option a person takes**
//! rather than the default, so the eye [`Body::eye`] is derived from a body that cannot be where the drawn
//! world is not; `controller`: **a person drives the client** — the session of those steps, one owner each for
//! the world (the law's own state), the resident set (the streamer's) and the eye (`viewpoint`'s), and the
//! run's own measured line for a walk that comes home). `evidence` measures each
//! built system's bar; the registry in `systems` is the honest scope list.

// `deny`, not `forbid`, since §36 (2026-10-13): a window **is** a platform call and a platform call is
// `unsafe`, so exactly one module — `surface::platform` — carries `#[allow(unsafe_code)]` and every other
// module in this crate is under the same denial it was under before. `forbid` cannot be narrowed, and a
// rule that cannot be narrowed would have forced a second crate for one door; the licence is one line and
// `grep -rn "allow(unsafe_code)" src` finds it. Nothing else changed: this crate still declares `sha2`
// alone as a dependency, and no windowing crate is added.
#![deny(unsafe_code)]

pub mod audio;
pub mod automaton;
pub mod axis;
pub mod being;
pub mod body;
pub mod bridge;
pub mod chain;
pub mod chunk;
pub mod controller;
pub mod coordinates;
pub mod digest;
pub mod drawn;
pub mod evidence;
pub mod feeling;
pub mod field;
pub mod gameplay;
pub mod intake;
pub mod laya;
pub mod light;
pub mod material;
pub mod measure;
pub mod mesh;
pub mod noise;
pub mod policy;
pub mod port;
pub mod presentation;
pub mod process;
pub mod shapers;
pub mod profile;
pub mod reference;
pub mod refusal;
pub mod save;
pub mod shaper;
pub mod shell;
pub mod stage;
pub mod stream;
pub mod surface;
pub mod systems;
pub mod transport;
pub mod viewpoint;
pub mod von;
pub mod world;
