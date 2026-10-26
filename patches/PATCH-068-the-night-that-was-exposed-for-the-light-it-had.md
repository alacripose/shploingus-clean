---
patch_id: PATCH-068
title: The night that was exposed for the light it had — one exposure for every frame, and the Moon's own beam as the only lift
status: applied
operation: fix
target: shploingus/exposure-grade
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-066, PATCH-067]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the reflected moonlight is as bright as the daylight, at night"
live_mutation: false
---

# Change

**The exposure was a tracking exposure, and a tracking exposure cannot show that the light is falling.**
PATCH-067 left `peak / max(peak · twilight, moon)` in place: the frame divided the light that was
arriving back out of itself. That is what an auto-exposure does, and it is why the night was drawn as
a day. Measured through the real path on the night the report came from — the Sun `8.6°` under the
horizon and the Moon `5.7°` up — the grade was **`1.5e3`**, and with the Moon at its height it reached
**`2.2e4`**. The Moon's own aureole is the brightest direction in that sky and the one the ambient
reads, so it was multiplied until it clipped white and the ground came with it: a flat, saturated
orange frame with the night's sky in it.

**The exposure is now the day's own, for every frame.** `EXPOSURE_GRADE` is `1.0` — the world's own
peak sun, the normaliser every brightness in `skylight.rs` was measured against — and it does not move
with the hour, the body or the phase. A night is dark because a night is dark. What lifts it is one
thing and one thing only: **`MOONLIGHT_GAIN`**, `5e3`, multiplied into the Moon's own measured beam
(`moon_illuminance / peak`) inside `daylight_of`. The measured ratio is `~1.3e-6` of the Sun's, which
at one exposure draws a black ground, so the gain is the declared lift that makes a night walkable:
it puts the moonlit ground at about **`3e-2`** against the day's `2.0`. Phase and altitude still show
through, because the measured illuminance is what is multiplied.

**The beam fades in rather than switching on.** `moonlight_fade` eases the night light over the first
`6°` of the Sun's setting with a smoothstep, so the tick the Sun's disc crosses `0°` no longer adds a
full night light to a twilight that is still lit — the one step PATCH-067 named as its own remaining
defect.

**And the ambient's horizon reading is a mean of four azimuths.** It was sampled at the Sun's own
azimuth — the brightest direction in a single-scattering sky and, near the horizon, the Sun's own
aureole: at a `4.9°` Sun it measured `1.6` against the noon's `0.23`, seven times noon, which is the
flat orange ground the report's first frame shows. The dome's mean does not spike when the Sun sets;
one direction of it does. Four rays `90°` apart carry that aureole at a quarter weight, and at noon the
mean is the one sample.

# Acceptance

- **No hour is lifted** — `skylight::tests::the_frame_is_exposed_once_at_every_hour_and_never_lifted`
  sweeps a whole day and a lunar month: the grade is `1.0` at every tick of the day, and the brightest
  night's drawn ground is measured against the day's own peak.
- **A night is a share of the day, not a second day** — the same test asserts the brightest moonlit
  night is **`1.45 %`** of the day's ground (bound `5 %`) and prints the tick and the Sun's altitude it
  was measured at.
- **The Moon's light is still the Moon's** — `the_shipped_night_opens_on_a_full_moon_and_the_ground_receives_its_light`
  now prints both numbers: the **measured** moon-to-sun ratio (`~1.3e-6`, under the published `1e-4`)
  and the **drawn** one (`~2.8e-2` at the gain, under a quarter).
- The drawn Moon's own radiance is unchanged and still saturates: the disc is white against a dark sky,
  and the light it casts is what lights the ground.

# Evidence

`cargo run -p voxel-client --example night-check` — the drawn ground, per tick, against the noon:

| tick | Sun | Moon | grade | of noon | display |
|---|---|---|---|---|---|
| 15750 | `4.9°` | `-7.0°` | `1` | `54.5 %` | `0.403` |
| 16500 | `-1.6°` | `-0.9°` | `1` | `1.87 %` | `0.023` |
| 17250 | `-8.6°` | `5.7°` | `1` | `1.60 %` | `0.019` |
| 18750 | `-23.8°` | `20.1°` | `1` | `2.69 %` | `0.032` |
| 21000 | `-47.5°` | `42.9°` | `1` | `3.09 %` | `0.037` |

Before the change the same tool read the night's ground at **`8.5 %`** of the noon *and* the twilight's
at a grade of `1.5e3`; the frame the user sent was the second of those.

- `cargo test -p voxel-client --lib` — **537 passed, 0 failed, 2 ignored** (the two failures this
  pass found in the orbit's own tests are repaired here, see below).
- The whole build is on `F:/shploingus-target` now, and `target/` on `C:` is gone.

# What this does not touch

- **Shadow sampling**, the tree mob, the gate, the walk and the cell-shell refusal: untouched, and the
  shadow sampling is the next thing the user points at.
- **The sky's own double count** named open in PATCH-063: still open. The sky is drawn at
  `SKY_EXPOSURE` and the ambient reads it back through the same number, so the two are one model; what
  is not settled is the *field's* stencil against the rig's two rays.

# Rollback

Restore `exposure_grade(peak, sun_altitude_deg, moon_illuminance)` as
`(peak / max(peak · twilight, moon)).clamp(1.0, NIGHT_EXPOSURE_SHARE · peak / moon)` with
`NIGHT_EXPOSURE_SHARE = 1/16`, drop `MOONLIGHT_GAIN` and `moonlight_fade` from the Moon's light, and
restore the single-azimuth horizon reading in `daylight_of`. The night is exposed for the Moon it has
again and the frame is a day.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0959
site: crates/voxel-client/src/skylight.rs:186
what: expose every frame once at the world's own noon, lift a night only through the Moon's own measured beam at a declared gain, and read the ambient's horizon as a four-azimuth mean instead of the Sun's own aureole
because: the tracking grade reached 2.2e4 on a moonlit night and 1.5e3 at a Sun 8.6 degrees under, so the sky's brightest direction was multiplied until it clipped and the ground came with it — a night drawn as a day
check: cargo test -p voxel-client --lib the_frame_is_exposed_once -> failures 0
witness: crates/voxel-client/src/skylight.rs
expects: 0
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace
```
