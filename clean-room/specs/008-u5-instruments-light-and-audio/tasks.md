# Tasks: U5 — Instrumentation, light and audio

**Input**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Record**: [quickstart.md](./quickstart.md)
and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §24.

**All tasks ran and are done** in one pass (2026-10-12, fourteenth pass); the measurements each produced are
in quickstart.md. Where a task named a file and the build put the thing elsewhere, the difference is noted
under the task rather than smoothed away.

- [x] **T001** The instruments, in a new module: the one sensor concept (`Sensor`, `Quantity`), the
  per-item `Band` with three words, `GaugeReading`/`Gauge`/`GaugeWall`, `Crossing`/`Mass`/`WeighingGate`.
  *Landed in `src/measure.rs`*, the four gate strings quoted in its header. **Corrected**: this plan had
  first named `src/intake.rs`'s neighbourhood for the vocabulary; U6's module is untouched, because the
  weighing gate's vocabulary is U5's own.
- [x] **T002** The four refusals, in `src/refusal.rs`'s closed vocabulary: `not_weighable`, `not_a_count`,
  `band_inverted`, `unwatched` — with `ALL`, `COUNT` and `name()` moved together and the round-trip test
  still holding every name.
- [x] **T003** The band table and the solver, in `src/light.rs`: `Channel`/`Bands`/`Air`/`Droplets`/`State`/
  `Spectra`/`Solver`, the closed form, two media two geometries, the twilight curve and its floor, the Moon
  as a reflectance, the response's two calibrations.
- [x] **T004** The audio seam, in `src/audio.rs`: C-94's six constants, `onset`, `Weather` (value, window,
  `gain`, `is_raining`, `is_wet`, `is_silent`) and `duty` — the field's own measurement.
- [x] **T005** `src/evidence.rs`'s `check_u5` (thirteen lines at §24; seventeen since the voices, §31) and
  `src/main.rs`'s join, so the bar is
  measured by the same function the binary prints and a bar not met exits non-zero.
- [x] **T006** `src/systems.rs`: U5's real status, its interface extended with `Crossing`/`Mass` and the
  seam's own two names — the registry is the list the binary prints.
- [x] **T007** `tests/u5_measure.rs`: six end-to-end tests through the public surface — the boundary audit's
  exact arithmetic, the wall and the gate as two consumers of one reading, the five refusals by name, the
  light's own claims, the audio seam, and the derivation documents' own presence.
- [x] **T008** The cascade: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 5 corrected in place and §24
  written; `docs/LEDGER.md` **C-186** filed; `clean-room/specs/008-…` created; the stdout line count moved
  from 444 to 457 with thirteen lines added and none removed *(§24's own reading; the voices then moved
  it to 494 → 498 with four added and three rewritten — §31)*.

## Findings this pass's own measurements produced

1. **A ridged onset field rained 0.9858 of its samples.** Named as a finding, the field replaced (rain as a
   positive anomaly), and the duty then measured at 0.0482 rather than bounded into passing.
2. **The sky was exempt from the droplet layer's own geometry**: with the scattering share read off per-metre
   coefficients, a load with no layer still changed the sky. The share now reads off the column's optical
   depths, and the no-layer case is asserted byte-for-byte.
3. **A twilight bound below the curve's own slope** (0.1% against 0.85/°): corrected and commented.
4. **`Band` briefly derived `Ord`**, which put `Ord::min`/`Ord::max` in scope beside the band's own ends;
   the derive was dropped and the reason is in the type's own doc comment.
