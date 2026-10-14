# Quickstart: U5 — how to see it run, and what it measured

## Run it

```bash
cd clean-room
cargo test --offline                              # 365 passed / 0 failed (295 lib + 70 integration)
cargo clippy --offline --all-targets -- -D warnings   # exit 0
cargo run --offline                               # exit 0, 498 lines of stdout, seventeen of them U5's
```

The binary prints the registry first (its own `[U5]` row and its `interface:` line were §24's two
pre-existing lines whose *text* moved; the voices then moved a third, the `derived from:` line — §31),
then each built system's measured bar. U5's own **seventeen** lines are the ones beginning `U5 `.

## The bar, quoted

`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 5, and the *U5* block of §2:

> `Sensor`/`Band`/`GaugeReading`/`GaugeWall` (a reading is a band, never a score), `Solver`/`Spectra`,
> and the audio seam. **A quantity that cannot be weighed is refused by name.**

## Scenario 1 — the instruments (SC-001, SC-002)

```
U5 instruments: 3 virtual gauges in one controller, band 32..=96 — 64→within, 31→below, 97→above
U5 weighing gate: 3 units of quartz = 7950000/1 g exactly; 64 units of basalt = 192000000/1 g (a mass, and nothing else)
U5 refusals by name: not_weighable=true (nothing crossed), unadmitted_material=true, not_a_count=true (a mass at a band), unwatched=true, band_inverted=true
```

A unit of stock is one voxel and the voxel is 1 m (`docs/DESIGN-SPEC.md` §5), so a count reaches the
density's own g/cm³ through `UNITS_TO_CM3` = 10⁶ and the arithmetic stays exact: quartz is `53/20`
g/cm³, three units are `3 × 10⁶ × 53/20 = 7 950 000 g`, reduced to `(7 950 000, 1)`. The three words
carry the band they were judged against, and there is no number between them.

## Scenario 2 — the table, the sky, the fog (SC-003)

```
U5 spectra: 12 bands over 400–700 nm, 1361.000000 W/m² (the solar constant); flat spectrum → (0.08333, 0.08333, 0.08333) spread 4.55e-7; the Sun → (0.080, 0.088, 0.086)
U5 sky: Sun 60°, ray 40° up → (0.039, 0.079, 0.143) — blue over red 3.6×, from λ^-4.09 alone
U5 fog: 0.021 g/m³ → 0.895 km visibility; the beam 1185.90 → 969.13 W/m² while the sky 21.2256 → 120.5584 — light moved into the line of sight
```

The Sun's bands are Planck at 5 778 K **scaled** so the twelve sum to the published solar constant —
a calibration, said out loud, and its target is **pinned as a literal** (`1 361 W/m²`): a moved constant
fails by name rather than scaling with the assertion. A flat spectrum is neutral because each channel is
divided by its own integral over the visible range (the residual `4.55e-7` is the quadrature's own seam error, measured
rather than assumed away). The blue is the record's own claim as a number: λ^-4.09, and nobody picked
it. The fog loads the sky while it hides the ground, because the droplets scatter 99% of what they
take.

## Scenario 3 — the twilight, the beam, the night (SC-004)

```
U5 twilight: 400.0 lx at 0°, 73.0734 at -2° (prints ~80), 13.3493 at -4° (prints ~15), 2.4387 at -6° (prints ~2.5), 0.0149 at -12° (prints ~0.01), 0.0010 at -18° (prints ~0.001)
U5 night: the curve meets the night sky's own 0.001 lx at -15.18°, before astronomical twilight's −18°; the floor as a radiance is 4.660e-7 W/m²/sr
U5 beam: 94.7526 W/m² at +0.1°, exactly 0.0 at −0.1° (not small: zero), and the sky at −18° reads 5.712e-6 against the night sky's own 5.593e-6
```

The twilight is `docs/missing_atmosphere_deepseek.md` §4.2's own anchored curve, each row inside a
factor of two of the value the reference prints, and the **beam is exactly zero** the moment the planet
occludes the body — the defect this whole slice exists to end was a model that returned zero *and a sky
that followed it*, so the sky keeps its floor: the scattered part at −18° is 2.14% of the night sky's
own measured radiance, and a night is dim rather than absent.

## Scenario 4 — the Moon (SC-005)

```
U5 moon: the full Moon 2.8519e-3 W/m² (1.95 lx at 683 lm/W, 0.29 lx at the Moon's own ≈100 lm/W), doubling the Sun doubles it (5.7039e-3), a tenth-lit crescent is exactly a tenth (2.8519e-4), an occluded Moon is exactly zero
U5 moon geometry: Ω = 6.4177e-5 sr = π(R/d)²; the legacy's own (R/d)² = 2.0428e-5 is π too dim, named rather than inherited
U5 sun ÷ moon: 4.772e5 (the record's own six orders: 4.5e5)
```

The Moon is **not an emitter**: its light is the Sun's own, reflected, so a dimmer Sun moves it and an
independent source would not. `0.29 lx` at the Moon's own ≈100 lm/W is the published full-moon figure —
and the 683 lm/W number beside it is a *warning*, because the Moon's spectrum is reflected sunlight whose
lumens-per-watt is nearer 100 than 683. The record's own measurement on its table is `3.0484e-3 W/m²`;
this build's is its own table's, in the same published zone, and the two are printed side by side rather
than fitted together.

## Scenario 5 — the audio seam (SC-006)

```
U5 audio: the onset read at (1234, −55) t7000 = 0.0290 (gain 0.000, raining false); one value per 2000 ticks and per 900 m; this field's own duty 0.0482 against the record's own measured 0.0997
```

C-94's own words are the reason it exists: the world's precipitation is **mm/year**, a long-run mean, so
nothing said when rain *starts* — *"an audio layer that wants is it raining right now had nothing to
read"*. The six constants are the record's; the field's own shape is this build's, declared: **rain is a
positive anomaly** of the tick-noise field (half the field dry by construction), sharpened by the
record's own dryness, scaled by a declared 2.0 for a unit mismatch — the thresholds are stated in the
field's units and an fBm's spread is not those units. The duty is then a **measurement** (0.0482) **pinned** to the figure this field returns, not a
bound raised until an assertion passed; a ridged field, tried first, rained 0.9858 of its samples and was
named as a finding rather than kept.

## What moved, and by how much (§24's own measurement, kept as the dated record)

| Quantity | Before | After |
|---|---|---|
| Library tests | 191 | **217** (+26: `measure` 9, `light` 12, `audio` 5) |
| Integration tests | 37 | **43** (+6: `tests/u5_measure.rs`) |
| Suite total | 228 | **260 passed / 0 failed** *(§23 recorded 226; the two more were not written by this pass and are named rather than guessed at)* |
| `RefusalKind`'s closed vocabulary | 61 | **65** (`not_weighable`, `not_a_count`, `band_inverted`, `unwatched`) *(§24's own; §31)* |
| `cargo run --offline` stdout | 444 lines | **457** (thirteen added, **none removed**) *(§24's own measurement — §31)* |
| Pre-existing lines whose text moved | — | **two**: the registry's own `[U5]` row and its `interface:` line |

Every other line is unchanged, checked against §23's recorded figures — the seven `U10` lines included,
with their digests `11873cad…` (the map) and `6ddffde0…` (the materials roster).

**The voices, and the current totals (2026-10-12, §30–§31).** The wheel and the voices moved the library
to **295** (12 of them `feeling.rs`'s, `audio.rs`'s and the band lookup's own), the suite to **365 / 0**,
`RefusalKind`'s closed vocabulary to **90** (`feeling_absent`, `feeling_undeclared`, `feeling_uncalibrated`,
`feeling_contradictory`), and the stdout to **494 → 498** — four lines added and **three rewritten** (the
`[U5]` row, its `derived from:` and `interface:` lines), seventeen `U5 ` lines, digest `b8203cbc…`. The
table above describes the build before them.

## What is owed, named

Ozone absorption; an **azimuth** (the solver is one plane through the Sun); the per-cell **frame format**
(`docs/light-solver.md` §6's remaining step, a renderer change); **the music layer** (§30 landed the
**voices** themselves — the feeling-wheel-driven bed and the rain — while the **material's** own voice and
**wind** stay owed, because they need the material row's acoustic columns and a surface geometry this build
does not carry); the wheel's own two source documents — `SKILL.md` and `reference.md` — which
`src/feeling.rs` cites but which live outside this repository, so the citation cannot be checked from the
tree (§31); and `WET_TURBIDITY_GAIN`'s link between a wet column and a hazier one. The four rulings U10 owes and `mc-ar-mod` **stand owed**, unchanged: they were
put to the user as questions, and no ruling was returned.
**The audit that followed (2026-10-12).** The delta's changed lines are **two**, named above; the solar
constant's calibration target is pinned as a literal, and `SOLAR_CONSTANT_W_M2` at `1 300`, `WET_ONSET` at
`0.26` and C-94's `0.18` row at `0.19` each fail by name — proved, then restored byte-exact; C-94's
constants are read as **values** and this field's duty is pinned to its measured `0.0482`; and the public
surface reachable only from inside its own module is cut (`scattering_per_m`, `view_tau`, `gauges()`,
`to_f64`, `with_air`/`hazy`, `GaugeWall::band`, `Channel`'s accessors), with `with_sun_factor`,
`Droplets::layer` and `Weather::is_silent`/`is_wet` refuted and kept. Nothing else moved: the suite is
260 / 0, clippy clean, the binary exits 0 at 457 stdout lines (13 added, 2 changed, 0 removed) *(§24's
own; §31)*, and the
four U10 rulings plus `mc-ar-mod` stay owed *(§24's audit on §24's build; the voices then moved the tree
to 365 / 0 at 498 stdout lines — §31)*.
