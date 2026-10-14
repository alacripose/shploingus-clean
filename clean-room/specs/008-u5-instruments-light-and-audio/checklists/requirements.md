# Requirements checklist: U5 — Instrumentation, light and audio

Read against `spec.md`'s FR-001…FR-011 and this pass's measurements (quickstart.md). Every box was checked
against a number in the binary's own output or a test that holds it, not against an intention.

## The instruments

- [x] **FR-001** One reading type: `Sensor` (`site`, `material`, `quantity`, `tick`); the wall reads it and
  adds no second reading. `GaugeReading` carries the band it judged against and the sensor it judged —
  `tests/u5_measure.rs::the_wall_and_the_gate_are_two_consumers_of_one_reading`.
- [x] **FR-002** `Band::of` refuses `min > max` by name (`band_inverted`), and `admits` answers three words
  with inclusive ends (band `32..=96`: `64→within`, `31→below`, `97→above`).
- [x] **FR-003** `WeighingGate` stands at one boundary; `Mass::of` is exact via `Density::grams_for` and
  `UNITS_TO_CM3` = 10⁶ (`3 units of quartz = 7950000/1 g`).
- [x] **FR-004** `not_weighable` for nothing crossed; `unadmitted_material` with the id named; `not_a_count`
  for a mass at a band (refused **before** the lookup the wall would otherwise make); `unwatched`;
  `band_inverted`. All five `true` in the binary's own line.

## The light

- [x] **FR-005** One band table; the Sun's bands sum to `1 361.000000 W/m²` by construction; a flat spectrum
  renders neutral to `4.55e-7` and the Sun to `(0.080, 0.088, 0.086)`.
- [x] **FR-006** The beam is exactly `0.0` at −0.1° and `94.75 W/m²` at +0.1°; the sky at −18° reads
  `5.712e-6` against the night sky's own `5.593e-6`, and the twilight reproduces the reference's table to
  its rounding (`400`, `73.07`, `13.35`, `2.44`, `0.0149`, `0.0010`).
- [x] **FR-007** The Moon is a reflectance: doubling the Sun doubles it (`5.7039e-3`), a tenth-lit crescent
  is exactly a tenth (`2.8519e-4`, to `1e-15`), an occluded Moon is exactly zero, and `Ω = 6.4177e-5 sr` is
  `π(R/d)²` — the legacy's own `2.0428e-5` named as π too dim.
- [x] **FR-008** Two media, two geometries: `0.021 g/m³` → `0.895 km`, the beam `1185.90 → 969.13` while the
  sky `21.23 → 120.56`; a **zero layer leaves both byte-for-byte untouched**.

## The audio seam

- [x] **FR-009** C-94's six constants quoted; one value per 2 000 ticks and per 900 m; the gain `0.0` at
  ≤ 0.05, `1.0` at ≥ 0.18, monotone between; **duty `0.0482` measured against the record's `0.0997`**,
  printed side by side.

## The standing rules

- [x] **FR-010** Every constant carries its source; the owed things (ozone, an azimuth, the frame format,
  the audio voices, `WET_TURBIDITY_GAIN`'s link) are named in the modules themselves.
- [x] **FR-011** `sha2` is still the only dependency; `find crates -type f -newer src/measure.rs` returns
  nothing (no file under `crates/` was touched or read as code).

## The suite

- [x] **SC-007** `cargo test --offline` — **365 passed / 0 failed**; `cargo clippy --offline --all-targets --
  -D warnings` — exit 0; `cargo run --offline` — exit 0 at **498 lines of stdout** *(re-measured
  2026-10-12, §31; §24's own reading was 260 / 0 at 457)*.

## Owed, and not ticked

- [ ] Ozone; [ ] an azimuth; [ ] the per-cell frame format (`docs/light-solver.md` §6); [ ] the
  music/ambient beds (the voices landed §30); [ ] the wheel's own two source documents, which
  `src/feeling.rs` cites but which live outside the tree (§31); [ ] `WET_TURBIDITY_GAIN`'s wet-column
  link; [ ] U10's four rulings and
  `mc-ar-mod` (put to the user, no ruling returned); [ ] U6's remainder, untouched.
