# sky — the light, the air it crosses, and the cadence it is read on

**Status: `DocumentRole::Derived` — a reference, not authority.** The record is
`docs/LEDGER.md`; where this file and the record disagree, the record wins and this file is
wrong. An anchor that exists *only* here may not be cited by a feature: the build gate
refuses it, exactly as it refuses one that exists only in `docs/HANDOFF.md`.

This file exists because the code it describes is written in one-line docstrings by ruling
(`VOWS.md`: *"Docstrings can be NO LONGER than 1 or 2 sentences"*), and a rationale that
lives nowhere is a rationale that gets re-derived wrongly. It is where the sky's long-form
reasoning went when it came out of `crates/voxel-client/src/skylight.rs`.

## 1. The chain, and who owns each link

```text
voxel_shapers::weather     precipitation, the onset          Cadence::Water    (x4)
voxel_shapers::clouds      the air column: base, layers      Cadence::Water    (x4)
voxel_shapers::climate     moisture, season, ozone          Cadence::Daily
voxel_shapers::atmosphere  pressure and density with y      published table
voxel_appearance::sky      Rayleigh/Mie, the transmitted sun
voxel_client::skylight     Sun + Moon + ambient -> LightRig  Cadence::Water    (x4)
voxel_client::renderer     the rig and the dome table, uploaded
```

**Each field is read on the cadence of the stage that owns it, and held between.**
`Stage::Weather` and `Stage::Atmosphere` are both `Cadence::Water` (every 4 ticks) while
`Stage::Climate` is `Cadence::Daily`, so a single "recompute the sky when the atmosphere is
due" would drag the ozone along at the weather's rate and make a daily quantity vary five
times a second. The client holds `rig_at_tick`, `turbidity_at` and `ozone_at` separately for
that reason.

**A stage's period is part of the determinism contract, not a performance knob** — the world
is a function of `(seed, tick)`, and the rig must be too. `the_rig_is_pure_in_the_tick` holds
that, which is also why no temporal filter may be used anywhere in this chain: a filter makes
the rig a function of its own history.

## 2. What the rig carries, and the four wrong answers

`Light` is `colour` x `intensity`. `LightRig::standard` settles which factor carries what:
colour `(0.85, 0.90, 1.0)` — a chromaticity whose strongest channel is 1 — and intensity
`1.0`. Every mistake in this file's history is one of those two factors carrying the other's
job.

| what was measured | the defect |
|---|---|
| `mean 0.87457`, brightest pixel `78.3` | `Light.colour` was `sky.sun` — a radiance of about **1090** — where the shader wanted a chromaticity; the ambient was `4.61765` where it wanted a ratio. **103x too bright.** |
| `mean 0.00085`, `0.8 %` of pixels above `0.01` | both corrected, and physically correct radiance is an invisible picture: W/m^2 and an 8-bit display share no scale and nothing between them declared one. |
| the normaliser divided by a sun that had not crossed this world's air | the divisor was the *geometric* irradiance at the year's highest noon while the dividend was the sun **after** `exp(-beta*H*m)`. Both are W/m^2 so nothing looked wrong; **the missing air column was 9.93 %** of every intensity the rig had ever produced. |
| the sky and the light met different exposures | the sky background and the light it casts come from one `Sky`, so they must meet one exposure. A sky drawn at one scale and lit by a light at another is the 103x defect in different clothes. |

`EXPOSURE = 4.0` is applied at **upload**, to the rig and to the sky colour, and never inside
`daylight_of` — so the rig stays a physical measurement in `[0, 1]` and
`the_rig_is_one_unit_so_no_field_of_it_is_a_raw_radiance` keeps its meaning. A scale folded
into the rig would be indistinguishable from the 103x bug, which was a scale folded into the
rig. It is a declared choice, re-measured rather than retained (`C-158`'s move for
`SKY_EXPOSURE`), and pinned by a test so a later edit has to say it moved it.

## 3. Turbidity, and the step while walking

`sky_inputs_at` gives Preetham's two inputs, **derived from the world rather than declared
beside it**: turbidity from `weather`'s precipitation *rate* (the onset layer's answer,
`C-94.7` — the annual mean was the figure the onset exists *beside*), and the ozone depth
from `climate`'s season.

**The rate is not a gentle quantity.** `weather::precipitation_parts`' upslope branch carries
`UPSLOPE_GAIN_MM = 26_000` against `BASE_PRECIPITATION_MM = 320` — a factor of **81** — on an
input that is a finite difference of the terrain's own elevation. So the rate is, to within
the moisture field, *the terrain's slope*. Dividing by `ONSET_DUTY` (0.0997, another x10)
through `WET_TURBIDITY_GAIN` (x3) left turbidity inheriting the terrain's **curvature**,
amplified about five thousandfold.

Measured with `cargo run --offline -p voxel-client --example exposure-steps`, walking 0.86 m
between two `Cadence::Water` updates:

| | before | after the footprint |
|---|---|---|
| worst turbidity step | `2.8064` (76 % of the line's mean) | `0.5693` |
| worst sun-intensity step | `0.098316` — **a fifth of the frame's light in a fifth of a second** | `0.017493` |

`EXPOSURE` never moved; the *illuminant* was stepping. That is the whole of the reported
"abrupt changes in exposure while walking".

**The repair is `TURBIDITY_FOOTPRINT_M = 64.0`** — the rate averaged over a disc (centre plus
4 rings of 8) before `turbidity_of`. Preetham's `T` is an aerosol load on *the column of air
the light passes through*: kilometres wide and eight tall. Reading it from the square metre
under the player's boots is a scale error of three orders of magnitude, and no filter over
time repairs a scale error — it only spreads it out, and it would break the purity rule in
§1.

**The footprint is a declared sampling choice and a stopgap, not the fix.** The size is a
small fraction of `weather`'s own `ONSET_CELL_M` (900 m), so the sky still tracks local
weather, and large compared to a walker's step between updates, which is the point. The fix
it stood in for is §5, and it landed — the footprint is gone as of the column slice.

### 3.1 The column, and the defect the wiring exposed

`sky_inputs_at` now reads `voxel_shapers::clouds::column_water_path_m`. That is **closed
form, not a march**: `cloud_at`'s profile is `water · sin(pi · through)`, whose integral over
the layer is exactly `water · 2 · (top - base) / pi`. One derivation per atmosphere update,
against the footprint's 33 point reads.

**`WATER_PATH_REFERENCE_M = 100.0` was measured, not chosen**, by
`examples/aerosol-column.rs`: over 1024 columns at four times of day the path runs `0.0` to
`125.0` m with a p90 of `33.5`, and **724 columns carry no cloud at all** (the model's own
29.3 % cloudy fraction). `100.0` puts this world's p90 column at `T = 4.01` and its deepest at
`9.50` — Preetham's published ordinary range rather than beyond it.

What it buys, measured on both axes at once (`0.86` m of ground per `Cadence::Water` update):

| the walk | turbidity step | sun intensity step |
|---|---|---|
| the spot rate, before any repair | `2.8064` | `0.098316` |
| the 64 m footprint | `0.5693` | `0.017493` |
| **the column** | **`0.2609`** | **`0.012982`** |

**And it introduced one, which the walking test does not catch because the walking test does
not stand still.** On the time axis at the origin the column source steps **`9.9598` of
turbidity and `0.328661` of the sun's intensity — 119 % of the day's mean light — in one
0.2 s update**, at tick 14428. Under the footprint the same measurement was `0.0586` and
`0.003688`.

**The cause is a boolean, and it is named in `cloud_column` itself.** A column's existence is
gated — `!witnessed && relative_humidity < CLOUD_FLOOR_RH` returns `None`, `!witnessed &&
lcl_m > lift_m` returns `None`, and `witnessed` is `precipitation_now(...).is_raining`, which
is `onset >= WET_ONSET`. So the moment the onset crosses `0.18` the cloud comes into
existence at **full** water, and a sky reading it inherits a step the size of the whole
cloud. The witness rule is worth its measured 299-of-299 corroboration as a *cloud* rule; as
a *light* input it is a discontinuity with no ramp, and the reader must not pretend
otherwise.

**The witness is now a weight, not a switch** — `water = relative_humidity · max(lift_ramp,
onset_weight)`, where `onset_weight` is the same `(onset - SILENT_ONSET) / (1 - SILENT_ONSET)`
normalisation `rain_fill_at` already uses and `lift_ramp` is the `1 - lcl/lift` the
non-witnessed arm already computed. `max` of two continuous ramps is continuous. Measured
afterwards, on the same day:

| the origin, per update | before the ramp | after |
|---|---|---|
| the two `is_raining` flips | `166.00` m then `158.61` m | `22.85` m then `21.78` m |
| the largest non-flip step in the day | `0.53` m | `3.96` m |
| ratio of flip to ordinary step | **~313x** | **~5.8x** |
| worst turbidity step | `9.9598` | `1.3709` |
| worst sun-intensity step | `0.328661` | **`0.067954`** |
| the aggregate water path | mean `8.2`, p90 `33.5`, max `125.0` m | mean **`8.5`**, p90 `33.5`, max `125.0` m |

So the cloud did not dim: `every_raining_column_is_cloudy` still holds by measurement rather
than by assertion, and the path distribution barely moved.

**The residual is now isolated, and it is the other gate.** `22.85` m is no longer the
witness *weight* stepping — it is `None` becoming `Some`. When `witnessed` turns true, both
existence gates stop applying, so a column that was not a cloud at all becomes one holding
`onset_weight · relative_humidity` of water. The existence decision is still a boolean, and
the light inherits whatever it decides. **A continuous existence needs a continuous
admission** — `cloud_column` returning a small non-zero path for air that is nearly a cloud
— which changes what the renderer's `None` means, so it is a ruling and not a repair.

One bound to watch while that is open: at a **fixed tick** the worst spatial step over the
walk is `0.048641` of the sun's intensity, which is **above** the `0.03`
`the_sky_does_not_step_across_ground_while_walking` declares — that test measures `0.012982`
and passes because of which 256 m it happens to walk. The bound is satisfied by the sample,
not by the model.

## 4. The horizon step, and where the ambient's worst step now is

The ambient is a **step function of solar altitude with its edge exactly at 0`deg`**, because
`Sky::dome_radiance` returns zero below the horizon. Documented in the source as "the named
absence", so it is a decision and not an accident, and
`the_sky_table_is_black_at_night_and_the_sky_below_the_horizon_clamps` pins it.

Measured under the footprint, standing still at the origin: one update in a whole day steps
the ambient by **0.287846 — 1.1514 after `EXPOSURE`** — localised to the crossing itself,
sun altitude **-0.021`deg` -> +0.020`deg`** at tick 29452. So the world's ambient snaps on at
dawn and off at dusk by more than full scale.

**Under the column source the worst ambient step has moved off the horizon entirely** —
`0.176711` (`0.7068` exposed) at tick 15132 with the sun at **+9.660`deg`**, which is §3.1's
boolean surfacing in the ambient rather than the horizon's absence. The horizon's own step is
not gone; it is no longer the largest one, and a reader comparing the two must measure rather
than assume which is which.

Nothing here repairs it. The physical fact is that a sky at -0.02`deg` is not black; civil
twilight runs to -6`deg`, where illuminance is roughly a thousandth of sunset's. But changing
it is a look decision, the look is the user's, and this document is not authority.

## 5. What is modelled and not yet wired

**This is a wiring job, and every system it needs already exists.**

`voxel_shapers::clouds` carries the air resolved: `cloud_column(spec, x, z, tick)` gives the
column, `cloud_at(spec, x, z, y_m, tick)` gives the air **at an altitude**, `surface_air` the
surface case, on Magnus–Tetens saturation with the lifting condensation level as cloud base,
`TROPOPAUSE_M` above and the dry/moist lapse pair between. Its own doc names what is absent:
no CCN activation, no collision–coalescence, no ice pathways and **no cloud-water balance
equation**.

So the sky should not read a rain rate at a point. It should integrate the column the sun's
own ray crosses — `cloud_at` along the ray — which is the quantity Preetham's `T` is about,
and which makes `TURBIDITY_FOOTPRINT_M` unnecessary rather than merely smaller. `C-152.2`'s
fluid query (`voxel_shapers::fluid_state`) and `column::derive`, which runs the whole stage
graph in the graph's own order, are the other two ends of the same wire.

`planet-shploingus` records the target in `docs/FLUIDICS.md` (SIGNED 2026-10-06): F1 makes the
32^3 subvoxel lattice the address space for substance, fluid **and gas** alike; F10 makes fog
**emergent** — aerial density resolved in the cells and interpolated, never a screen effect;
and F11 binds it to the light, because `LIGHT-CONTRACT.md` L15 rules that a cell's optical
value comes from its **subvoxel mix** and L14 that when the atmosphere lands the per-band
coefficients come from the cells. One field sampled by the sim and the renderer, not two.

## 6. Open questions

1. **§3.1, and it is the live one: what admits air that is nearly a cloud?** The witness
   weight is continuous now; the *existence* decision is not, and it is what the remaining
   `22.85` m step is made of. A continuous admission changes what `None` means to the
   renderer, so it needs a ruling before it is a repair. Nothing here picks one.
2. §4: does the horizon step get a twilight, and if so by whose ruling?
3. §3.1: `WATER_PATH_REFERENCE_M` was measured against a world whose deepest column is
   `125.0` m. If the gate becomes continuous, the deepest column changes and the reference is
   owed a re-measurement rather than a new guess.
4. §5: does the reference or a footprint return as the fallback for a column the model
   refuses?
5. Does this document get declared in `voxel-package`'s ten-document set, or stay a
   derived reference like `docs/HANDOFF.md`? Adding it changes a declared set.
