# Plan 010 — U8 The shell (the window, the mount, the design system)

## 1. Summary

Implement the shell as **one dependency-free Rust module**, `src/shell.rs`: the admitted constants table
(the four buckets), the declared `Recipe` and its `WindowSpec` windows, the `Mount` pane tree with the
shell/HUD layer toggle, the one pointer-ownership decision, a `Canvas`, and the `frame` that draws a
visibly bordered, readout-bearing frame. A `shell` mode in the binary makes the frame interactive. The
module adds **no** dependency and imports **nothing** from `crates/` (P7).

## 2. Technical context

- **Language / edition / toolchain:** Rust, edition 2021, workspace-pinned toolchain; the crate's only
  dependency is `sha2` (content identity), unchanged.
- **Target:** a **character grid**. There is no GPU and no windowing library, and this build adds none:
  the frame is drawn into a `Canvas` of chars and printed, which is what makes *"a visibly drawn frame"*
  a thing a person looks at. The cell's pixel size is therefore a **platform translation** row in the
  constants table, declared rather than implied.
- **Testing:** unit tests in `src/shell.rs` (eleven), the closed-vocabulary round-trip in
  `tests/u5_measure.rs`, and the binary's own evidence (`evidence::check_u8`).
- **Constraints:** refuse-never-default; determinism (the frame is a pure function of the mount's state);
  no legacy import; no dependency; **one** concern per owner.
- **Scale:** the declared frame extent is **100 × 30** characters (`shell::declared_area`), shared by the
  `shell` mode, the evidence and the tests, so the frame is reproducible and comparable by digest.

## 3. Constitution Check

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | `src/shell.rs`'s header names §2 (U8), §5 row 7, `SHELL-PORT.md` §1/§4, `UI-SPEC.md` §1–§3/§8/§9, `DESIGN-SPEC.md` §3; every constant row names its source |
| P2 Refuse, never default | PASS | `constant_unadmitted`, `pane_unknown`, `split_floor_breached` — the three U8 kinds; an unreadable system's refusal is printed by name in its pane |
| P3 Determinism is a contract | PASS | `the_same_state_draws_the_same_frame`; the frame's digest is `Digest::of` over the drawn cells |
| P4 Evidence over assertion | PASS | `check_u8` prints the frame's size, its digest, the bucket counts and the two booleans; SC-001…SC-004 each name a check |
| P5 One declared interface, two doors | PASS | one `Mount`, one `frame`; the binary's report and the `shell` mode both call them |
| P6 Smallest sufficient artifact | PASS | one module and one mode; no stub, no second tree, no second registry |
| P7 Legacy is a witness, never a source | PASS | `clean-room/` has its own `[workspace]`; no `crates/*` path dependency; the witness (`crates/shploingus`, `crates/voxel-ui`, `shell/`) was not read |
| P8 https://github.com/gholtzap/lintapple | WIP | NEEDS WORK — **U13's**, not this slice's; see §5 below |
| P9 https://github.com/mikuta0407/win4wine | WIP | NEEDS WORK FOR SHELL — shell *packaging*, not the shell; untouched |

## 4. Project structure

| File | Owns | Lines |
|---|---|---|
| `src/shell.rs` | the whole slice: `Bucket`/`Constant`/`CONSTANTS`/`constant`/`bucket_counts`, `Lane`, `Content` and its six readers, `WindowId`/`WindowSpec`/`Recipe`, `PaneId`/`Orientation`/`Node`/`Mount`, `Layer`/`PointerOwner`/`Pointer`, `Area`/`Canvas`/`frame`, and eleven tests | **1406** |
| `src/refusal.rs` | the three new kinds (`constant_unadmitted`, `pane_unknown`, `split_floor_breached`) and the count **77 → 80** | 623 |
| `src/evidence.rs` | `check_u8`, appended beside the other checks | 2935 |
| `src/main.rs` | the `shell` mode, `apply_command`, and U8's line in the gate | 236 |
| `src/systems.rs` | U8's status, interface and witness | 328 |
| `tests/u5_measure.rs` | the pinned count **80** and the three names | 359 |

**Created**: `src/shell.rs` (**1406** lines) — re-measured on the final bytes (§27 of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md`). **Changed**: `src/refusal.rs`, `src/evidence.rs`, `src/main.rs`,
`src/systems.rs`, `tests/u5_measure.rs`.

## 5. The `gholtzap/lintapple` gate — the slice that takes it

This plan's own P8 row is `WIP | NEEDS WORK`, and it **stays** so: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5
row 13 defines **U13** as *"the shell completed, and its gholtzap/lintapple gate"*, and
`clean-room/src/systems.rs`'s U13 entry names it. U8 is the shell; U13 is *the shell completed* plus the
gate, and it is last by §2's own reasoning. Taking the gate here would duplicate U13's bar. What this
slice contributes to the gate is the half the gate checks — **every constant admitted** — which is the
constants table, refused by name when a name is not in a bucket.

## 6. Complexity tracking

*(empty)* — no principle is violated. The deliberate divergences are recorded, not hidden:
(1) the drawing target is a **character grid** rather than a GPU surface, because adding a windowing
dependency would break the offline, dependency-free discipline this crate keeps — recorded as a
`platform-translation` constant rather than left implicit;
(2) **undock**, the splitter-drag resize, the console's five graded levels and the toast's final form are
**owed by name** rather than half-built;
(3) the initial tree is a **grid** (columns of three, side by side) rather than a flat list, so the six
declared windows fit a normal frame above the 40 × 40 pane floor.
