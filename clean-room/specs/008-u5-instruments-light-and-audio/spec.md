# Feature Specification: U5 — Instrumentation, light and audio

**Feature Branch**: `008-u5-instruments-light-and-audio`

**Created**: 2026-10-12

**Status**: **Built (2026-10-12, fourteenth pass).** `clean-room/src/measure.rs`, `src/light.rs`,
`src/audio.rs`, `src/evidence.rs`'s `check_u5` and `tests/u5_measure.rs` are in the tree; the measured
numbers are in [quickstart.md](./quickstart.md) and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §24. The
slice's own record: **365 tests passed / 0 failed**, clippy clean,
`cargo run --offline` exit 0 at **498 lines of stdout** *(§24's own reading was 260 / 0 at 457; the
voices then moved both — plan §31)*, `crates/` untouched.

**Input**: User description: "U5's own bar — instrumentation, light and audio: the one sensor concept,
the per-item min/max band, the weighing gate, the gauge wall; the light solver and the band table;
the audio seam — with a quantity that cannot be weighed refused by name."

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5's row 5 and the *U5* block of §2, §16.2's order, and §24.

## What U5 is, and what it is not

U5 is the **measuring layer**: one concept of *a reading*, the instruments built from it, and the two
things this world reads light and weather out of — a **band table** with a closed-form solver, and the
**onset field** an audio layer asks *is it raining right now* of. It is **not** a port of the legacy's
`voxel-instrument`/`voxel-light`/`voxel-audio` (P7): their behaviour is evidence, their code is not
read, and where the legacy's arithmetic is wrong (the Moon's missing π) the defect is **named** rather
than inherited.

Four boundaries are inherited whole:

1. **One sensor concept** (S-03's gate string). A gauge and any overlay are two *consumers* of one
   reading, never two instruments.
2. **A band is per item, with min and max** (S-12's). A mass is not a count of an item, and a
   judgement is three words — never a score a caller could threshold on its own terms.
3. **One weighing gate per boundary, and its answer is a mass and nothing else** (S-13's). The type
   has no room for a count or an item name.
4. **Refuse, never default.** A quantity that cannot be weighed is refused **by name**; so is a band
   with no inside, a mass at a band, and a reading no gauge answers for.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The instruments read, judge and weigh (Priority: P1)

A stock's count is read; the wall judges it against the band watching that item at that site; what
crosses a boundary is weighed through the material's own density, exactly.

**Acceptance Scenarios**:

1. **Given** a wall of virtual gauges in one controller, **When** a count is read, **Then** the answer
   is one of `below`/`within`/`above` **and carries the band it was judged against**.
2. **Given** a crossing with a count, **When** it is weighed, **Then** the answer is a `Mass` in exact
   grams (`count × density`, reduced) and the type carries nothing else.
3. **Given** a mass reading, **When** a band is asked to judge it, **Then** the refusal is
   `not_a_count` — S-12's rule, refused as a rule rather than as a lookup failure.

### User Story 2 - The sky, the beam, the twilight and the Moon (Priority: P2)

One band table; one closed form read for the sky, the beam and the Moon's reflected light.

**Acceptance Scenarios**:

1. **Given** the twelve bands, **When** the Sun's own bands are summed, **Then** the total is the
   published solar constant, and a flat spectrum renders neutral in the response.
2. **Given** a sun below the horizon, **When** the beam is asked for, **Then** it is **exactly zero** —
   and the sky is **not**: it decays on the reference's own curve to the night sky's own measured
   radiance.
3. **Given** a droplet load, **When** its layer is declared, **Then** the sky is loaded while the path
   is darkened, and **no layer declared is no path at all**.

### User Story 3 - The audio seam answers "is it raining right now" (Priority: P3)

The world's precipitation is mm/year — a long-run mean — so nothing says when rain *starts*. The onset
field answers per place and tick, and the audio layer reads one number out of it.

**Acceptance Scenarios**:

1. **Given** a place and a tick, **When** the onset is read, **Then** the value is one window of
   2 000 ticks and one cell of 900 m, and is a function of nothing else.
2. **Given** a value, **When** the gain is read, **Then** it is `0` at or below the silence threshold,
   `1` at or above the wet threshold, and monotone between them.
3. **Given** the field, **When** its duty is measured, **Then** it is printed **beside** the record's own
   measured figure rather than fitted to it.

### Edge Cases

- Nothing crossed the boundary → `not_weighable` (no `0 g` is invented).
- A material the registry never admitted → `unadmitted_material`, with the id named.
- A reading no gauge watches → `unwatched`; a band whose min exceeds its max → `band_inverted`.
- A droplet load with no layer → the beam and the sky are **byte-for-byte** the clear ones.
- A mass at a network site → refused as `not_a_count` *before* any lookup, because the rule is about
  the reading and not about the wall's coverage.

## Requirements *(mandatory)*

- **FR-001**: `Sensor` MUST be the only way to ask what a thing measures; the wall MUST consume it and
  add no second reading type.
- **FR-002**: A `Band` MUST carry min and max, inclusive, and MUST refuse min > max by name.
- **FR-003**: A `WeighingGate` MUST stand at one boundary and its answer MUST be a `Mass` computed
  through `Density::grams_for` in exact rationals, with one unit of stock being one 1 m voxel.
- **FR-004**: A quantity that cannot be weighed MUST be refused by name (`not_weighable`), and each
  other case MUST have its own name (`not_a_count`, `unwatched`, `band_inverted`,
  `unadmitted_material`).
- **FR-005**: The light solver MUST read **one** band table, and the Sun's bands MUST sum to the
  published solar constant by construction.
- **FR-006**: The beam MUST be exactly zero below the horizon, and the sky MUST NOT be — the floor is
  the night sky's own measured radiance.
- **FR-007**: The Moon MUST be a **reflectance**: doubling the Sun doubles its light, a tenth-lit
  crescent is exactly a tenth, and its disc's solid angle is `π(R/d)²`.
- **FR-008**: Air and droplets MUST be two media over their own geometries; a declared zero layer MUST
  leave both the beam and the sky untouched.
- **FR-009**: The audio seam MUST answer from the record's own constants (C-94's table) with a field
  whose shape is declared and whose duty is **measured and printed** beside the record's.
- **FR-010**: Every constant MUST carry its source in its own doc comment, and every owed thing (ozone,
  an azimuth, the frame format, the audio voices) MUST be named in the modules rather than silently
  missing.
- **FR-011**: `sha2` MUST remain the crate's only dependency, and no `crates/*` path may be read (P7).

## Key Entities

- **`Sensor`** (new): the one reading — site, material, quantity, tick.
- **`Quantity`** (new): `Count(u64)` or `Mass(Mass)`.
- **`Mass`** (new): a raw mass number in exact grams, and nothing else.
- **`Band`** / **`Verdict`** / **`GaugeReading`** (new): the per-item interval, its three words, and the
  answer that carries its grounds.
- **`GaugeWall`** / **`Gauge`** (new): many virtual gauges in one controller.
- **`WeighingGate`** / **`Crossing`** (new): the one gate at a boundary and what crossed it.
- **`Channel`** / **`Bands`** / **`Air`** / **`Droplets`** / **`State`** / **`Spectra`** / **`Solver`**
  (new): the band table and the thing that reads it.
- **`Weather`** (new): the onset read — the value, its window, and the audio layer's own gain.

## Success Criteria *(mandatory)*

- **SC-001** Every count the wall judges is one of three words and carries its band; every mass the
  gate answers is exact. *Measured:* band `32..=96` reading `64→within`, `31→below`, `97→above`;
  `7 950 000/1 g` and `192 000 000/1 g`.
- **SC-002** A quantity that cannot be weighed is refused by name, and so is each other case.
  *Measured:* all five `true` in the binary's own line.
- **SC-003** The twelve bands sum to the solar constant and the response is neutral for a flat
  spectrum. *Measured:* `1 361.000000 W/m²`, spread `4.55e-7`.
- **SC-004** The beam is exactly zero below the horizon while the sky holds the night sky's own floor.
  *Measured:* `94.75 W/m²` at +0.1°, `0.0` at −0.1°, `5.712e-6` against `5.593e-6` at −18° (2.14%).
- **SC-005** The Moon's light lands where the published figures put it. *Measured:* `2.8519e-3 W/m²`,
  `0.29 lx` at the Moon's own ≈100 lm/W, sun ÷ moon `4.772e5`.
- **SC-006** The audio seam answers, and its duty is reported against the record's own. *Measured:*
  `0.0482` against `0.0997`.
- **SC-007** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is
  clean, `cargo run --offline` exits 0. *Measured (2026-10-12, §31):* 365 / 0, exit 0, exit 0 at 498
  stdout lines — §24's own reading of this gate, before the voices, was 260 / 0 at 457.

## Assumptions

- **What U5 owes is named, not silently missing**: ozone absorption, an azimuth, the per-cell frame
  format (`docs/light-solver.md` §6), the material's own voice, wind and the music layer, and
  `WET_TURBIDITY_GAIN`'s wet-column link. **The audio voices themselves landed in §30** (the nineteenth
  pass): the feeling wheel read as a value, and one bounded buffer out of it.
- **The field's own shape is this build's**; the record's constants are the record's, and the duty is
  measured rather than fitted.
- **The four rulings U10 owes and `mc-ar-mod` stand owed** — none is decided here.
- **What the sky, the fog and a material look like is the user's** (`docs/DESIGN-SPEC.md` ruling 4).

## Sources used, cited by path

| Path | What it supplied |
|---|---|
| `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U5), §5 row 5, §16.2, §24 | the system's definition, its interface, its bar, and this pass's own record |
| `docs/DESIGN-SPEC.md` §3 (S-03, S-12, S-13, FC-2.16), §5 | the four slots, their gate strings' home, and the scale contract (voxel 1 m) |
| `docs/LEDGER.md` C-25, C-93.5, C-94 | the four gate strings; the onset layer's ruling and its six-constant table |
| `docs/light-solver.md` §1–§6 | the closed form, two media two geometries, the twilight, the Moon's π, the droplet optics, the response's calibrations, and §6's owed format |
| `docs/missing_atmosphere_deepseek.md` §1.5, §4.2, §4.5 | Ångström's exponents, the anchored twilight curve, the night sky's own 0.001 lx |
| `docs/skylight.md`, `docs/lighting-sky-and-moon.md` | the legacy's own long form, read for behaviour only |
| `crates/voxel-instrument`, `crates/voxel-light`, `crates/voxel-audio` | **witnesses**: their surfaces name the vocabulary; no line was copied |
| `clean-room/src/{material,refusal,noise}.rs` | the exact density arithmetic, the closed refusal vocabulary, and the tick-noise field |
