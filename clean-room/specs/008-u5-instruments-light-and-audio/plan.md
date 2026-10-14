# Implementation Plan: U5 — Instrumentation, light and audio

**Branch**: `008-u5-instruments-light-and-audio` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-u5-instruments-light-and-audio/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7) and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2's *U5* block, §5's
row 5 and §16.2's order.

## Landed (2026-10-12, fourteenth pass)

The execution order below ran in one pass. `src/measure.rs` (554 lines), `src/light.rs` (911),
`src/audio.rs` (271) and `tests/u5_measure.rs` (297) are in the tree, with `src/refusal.rs` gaining four
kinds, `src/lib.rs` the three modules, `src/evidence.rs` `check_u5` (thirteen lines at §24; **seventeen**
since the voices, §30–§31), `src/main.rs` the
bar's join, and `src/systems.rs` U5's real status. No dependency was added (`sha2` remains the only one),
no file under `crates/` was read, and the three witness crates were read for **behaviour only** — the gate
strings C-25 carries and the published tables — with no line copied.

Measured: **260 tests passed / 0 failed** (32 of them this pass's own), clippy clean,
`cargo run --offline` exit 0 at **457 lines of stdout** *(§24's own, dated)* (444 → 457; thirteen added,
none removed; the
registry's own `[U5]` row and its `interface:` line are the **two** pre-existing lines whose text moved) —
the voices then moved the tree to **365 / 0** at
**498** stdout lines with four added and three rewritten (plan §31). The full table is
[quickstart.md](./quickstart.md); the pass's own record is `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §24.

## Constitution check (P1–P7)

- **P1 evidence over assertion** — every number in evidence.rs is read back out of the system it belongs to,
  and a bar not met returns `false` so the binary exits non-zero. ✓
- **P2 refuse, never default** — four new named kinds, and no `Default` on any new type. ✓
- **P3 one owner per concern** — one reading type, one band table, one gate per boundary, one onset read.
  The exact-mass arithmetic stays in `material.rs`'s `Density`. ✓
- **P4 documents before code** — each module's header names the document it was derived from and the
  constants' sources. ✓
- **P5 measurement where a number is claimed** — the duty, the neutrality residual, the night sky's
  scattered share and the fog's two directions are all measured, not asserted. ✓
- **P6 compute the horizon, do not copy it** — the view and sun columns are computed (`column_tau`,
  `sun_transmittance`); no
  per-cell frame format is invented (named owed instead). ✓
- **P7 new code beside the old tree** — nothing under `crates/` was read as code; the witness crates name a
  vocabulary, not a body. ✓

## Structure decisions

1. **Three modules, not one.** `measure` (instruments), `light` (the table and the solver) and `audio` (the
   seam) have nothing to say to each other at the type level: the light does not consume a reading, and the
   audio does not sit at a boundary. Where they *do* meet — a tick and a place — they meet through the
   caller, which is what `tests/u5_measure.rs` proves.
2. **`Mass` lives in `measure.rs` and is built only by `Mass::of`.** The gate's answer is a mass and nothing
   else (S-13), so the type is one exact rational and one constructor, and `Density::grams_for` keeps the
   arithmetic.
3. **Two media, two geometries, one floor.** The air's column is `β_air · H · m`; the droplets' is
   `β_drop · D · m`, with `m` floored at the air-mass formula's own horizon value (37.92, which is what the
   record's declared `GRAZING_AIR_MASS = 38` rounds). That single decision is what makes a ray looking up
   leave the droplet layer quickly, and it is asserted both ways.
4. **The response's calibrations are the record's two.** A band's colour is an integral over the band on the
   geometric grid (the Jacobian is λ), and each channel is divided by its own integral over the visible
   range; an equal-energy white is then neutral, which the test measures (`spread 4.55e-7`).
5. **The onset field is declared and its duty measured.** C-94's constants are quoted; the field's shape
   (rain as a positive anomaly), its scale (2.0, for a unit mismatch) and the layer depth (40 m, for "tens of
   metres") are declared, and the duty is printed beside the record's rather than fitted.

## What this slice does not land, and names

Ozone absorption; an azimuth; the per-cell **frame format** (`docs/light-solver.md` §6); the music layer
(`§30` of the plan landed the **voices** themselves — the wheel's own bed and the rain — and names the
material's own voice and wind as still owed); `WET_TURBIDITY_GAIN`'s wet-column link. U6's remainder is untouched,
and U10's four owed rulings plus `mc-ar-mod` stand owed — put to the user as questions, no ruling returned.
