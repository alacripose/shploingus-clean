---
patch_id: PATCH-067
title: The twilight that was exposed for the Moon — the grade follows whichever body is arriving, and a moonless night stays dark
status: applied
operation: fix
target: shploingus/exposure-grade
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-066]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the rendered twilight is a hard step: the sky is blown to a flat white wall while the sun is gone, then the world goes black, so the twilight is not following the sun"
live_mutation: false
---

# Change

**The exposure grade was a blend of two terms and became a choice between them.**
`exposure_grade` was `(peak / moon_illuminance)^(1 − twilight)`: continuous in the Sun's altitude, but
meaningless where both terms are small. At `9.6°` under the horizon `twilight_fraction` is `3.0e-4`, so
the exponent is essentially `1` and **the whole twilight was exposed as if it were night**: measured
through the real path (`daylight_of` → `SkyModel::of_graded` → `sky_exposed_at`, the CPU's own reading of
the shader), that frame's horizon carried **`111`** against the day's own **`5.4`** — a flat, blown white
wall where a sunset should be — while at `1.6°` under the horizon the same sky was `0.79` behind a grade
of `1`. The sun was gone from the picture, the sky was lit like noon, and the sky had already been at that
grade since `-16°`: the twilight was a step.

**The law is now "expose for what is arriving"**: `peak / max(peak · twilight, moon)`, clamped to `[1,
peak/moon]`. While the Sun is up the arrival is `peak` itself, so every daylight frame is exactly `1.0`
and unchanged. Below the horizon the Sun's arrival is its own twilight share — the same curve the sky's
scattering rides — and the Moon's reflected light is the other body's arrival; the grade follows whichever
is brighter. The change lands only in the band where the Sun's twilight still outshines the Moon: the dawn
horizon at `-9.6°` went from **`111` to `1.07`**, a band with a gradient instead of a wall, and the night
frames (grade `3.6e5`–`5.2e5`) are untouched.

**The ceiling is the Moon's own exposure, and it is not decoration.** `peak / (peak · twilight)` alone
would rise without bound through astronomical night — `1e7` at `-30°` — and the night sky, whose radiance
is itself proportional to `twilight`, would be drawn at a constant **daylight** level: a white sky on a
night with no Moon. When the Moon is down there is no reference to lift to, so the grade is `1.0` and the
night is dark, as it was.

# Acceptance

- **The twilight is a band, not a noon and not a black frame** —
  `skylight::tests::the_twilight_is_a_band_between_the_day_and_the_night` sweeps the whole day at the
  cadence's own resolution, builds each tick's graded model through `daylight_of` and reads the horizon
  with `sky_exposed_at`, and asserts: the twilight's peak horizon (`2.578` at `-0.2°`) is **dimmer than the
  day's own** (`9.663`), **visible** (`> 0.2`, not a black frame), and a moonless night's is `4.3e-8` —
  dark. Under the old law the same sweep reads `111` in the twilight band and the first assertion fails.
- **The day was untouched on this record's tree — it is not untouched now.** The grade is exactly
  `1.0` for every tick the Sun is up (`SkyPhase::sun_is_up`), so the daylight frames were
  byte-identical **then**; the night frames with the Moon up carried `peak / moon`. PATCH-068 has
  since replaced the tracking grade with a fixed `EXPOSURE_GRADE = 1.0` at every hour and read the
  ambient's horizon as a four-azimuth mean rather than the Sun's own azimuth, so the night frames and
  the daylight ambient both moved. What stands is the grade while the Sun is up; the byte-for-byte
  claim belonged to this record's own tree.
- **The mapping was step-free on this tree** — `the_exposure_grade_follows_the_light_across_a_sunset_without_a_step`
  passed unchanged there, its `GRADE_STEP` bound and its printed worst step included, and its
  `night > 1.0` claim held because a moonlit night was `peak / moon`. **That test is gone**, because
  PATCH-068 removed the tracking law it measured: with the grade fixed at `1.0` there is no grade
  curve left to step, and the step this record named as its own remaining defect moved to
  `moonlight_fade`. `the_frame_is_exposed_once_at_every_hour_and_never_lifted` is the test that now
  holds the exposure still.

# What this does not touch

- **The sky double-count** named open in PATCH-063, **shadow sampling**, the tree mob, the gate and the
  walk: untouched.
- **The remaining step, named rather than hidden**: the Moon's own illuminance is exactly zero when it is
  below its horizon, so the night *at moonrise* still steps from `1.0` to `peak/moon`. That is a light
  that cuts and not an exposure that blends, and it is the next piece of this work — the doc on
  `exposure_grade` says so.

# Evidence

- `cargo test -p voxel-client --lib the_twilight_is_a_band -- --nocapture` — **1 passed**, printing:
  `horizon luminance: the day's 9.663, the twilight's 2.578 at -0.2°, a moonless night's 4.32e-8`.
- The rendered frames, measured offscreen through the game's own configuration
  (`SkyModel::of_graded` + `exposed_at` + tonemap): the horizon at `-9.6°` was `111` before and is `1.066`
  after; the dawn frame went from a flat saturated orange field to a graded sky with a glow at the
  horizon.
- `cargo test -p voxel-client --lib` — **536 passed, 0 failed, 2 ignored**.
- `cargo test -p voxel-shapers` — **362 passed, 0 failed, 1 ignored**.
- `cargo check --workspace` — **Finished**, no errors.

# Rollback

Restore `exposure_grade`'s `(peak / moon_illuminance)^(1 − twilight)` form with its `max(1.0)`, and delete
`the_twilight_is_a_band_between_the_day_and_the_night`. The twilight is exposed for the Moon again and the
dawn horizon reads `111`.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0958
site: crates/voxel-client/src/skylight.rs:171
what: expose for whichever body is arriving — the Sun's twilight share below the horizon against the Moon's own light — capped at the Moon's exposure, so the twilight is a band and a moonless night stays dark
because: `(peak/moon)^(1-twilight)` is essentially `peak/moon` already at -9.6°, so the dawn horizon was drawn at 111 against the day's 5.4 — a blown white wall with no sun in the frame — while a moonless night sat at a grade of 1
check: cargo test -p voxel-client --lib the_twilight_is_a_band -> failures 0
witness: crates/voxel-client/src/skylight.rs
expects: 0
requires: cargo test -p voxel-shapers
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace
```
