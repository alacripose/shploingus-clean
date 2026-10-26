---
patch_id: PATCH-065
title: The sun that went out at the horizon — the body is drawn while it emits, and its radiance is what fades
status: applied
operation: fix
target: shploingus/sun-body
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: []
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the sun body must not stop rendering below the horizon, because its light is what the Moon's face reflects and a night's shadow sampling names"
live_mutation: false
---

# Change

**The Sun's body was switched off at the horizon, and nothing else was.** `DrawnSun::of` returned
`None` when `SkyPhase::sun_is_up(altitude)` was false — a gate that had already been separated from
`Sky::sun_light` (which keeps the horizon's own air column at every hour below the horizon) and from the
rig (which takes the twilight share of it). So at `0.000°` the sky still had light, the Moon still had a
phase to draw, and the **source body disappeared** from the record stream: twilight was a sky full of
scattered light with no sun in it, and a night had no sun record at all for the Moon's face or a
shadow's source to name.

**The switch off is what is gone; what fades is the radiance.** `DrawnSun::of` now refuses only when
the model refuses — an emission that is not finite or not positive, or no normaliser — and its radiance
is scaled by `voxel_light::twilight_fraction(altitude)`, the same share `daylight_of` takes for the
rig's beam. That is `1.0` while the Sun is up (so the drawn disc is unchanged in daylight), `0.0060` at
−6°, `7.5e-9` at −22°: a body that dims through civil, nautical and astronomical twilight rather than
stepping out at the horizon, and is never a second light under the world.

Nothing about `BodyCuboid::at_tick` changes: its `BelowHorizon` refusal is the **shadow-casting** rule,
and where a body is stands apart from whether a shadow can be measured from it
(`the_shadow_rule_refuses_a_sun_the_drawn_body_still_places`).

# Acceptance

- **A body at every hour, and the fade is measurable** —
  `sun::tests::the_body_is_drawn_at_every_hour_and_its_radiance_is_what_fades` sweeps the world's own
  day: a body exists at all 96 samples; above the horizon its radiance is exactly `sky.sun` over the
  body's own solid angle (`1.0e-9` relative), and below it is positive and **less than** that — the
  share, not a second sun.
- **The direction claim survives where it is still true** —
  `sun::tests::the_drawn_body_points_where_the_rig_s_light_points` holds the body against the rig's
  first light while the Sun is up, and against the Sun's own direction below the horizon, where the
  rig's first light has become the Moon's.
- **The night frame still draws only the Moon** —
  `renderer::tests::the_moons_phase_is_drawn_from_its_own_geometry` now asserts the Sun's body exists
  and is below the eye while the scene carries the Moon alone, so its pixel comparison is unchanged.

# Evidence

- `cargo test -p voxel-client --lib sun::tests` — **16 passed, 0 failed**.
- `cargo test -p voxel-client --lib renderer::tests::the` — **18 passed, 0 failed**.
- `cargo test -p voxel-client --lib the_moons_phase` — **1 passed, 0 failed**.
- `cargo test -p voxel-client --lib` — **534 passed, 0 failed, 2 ignored**.

# Rollback

Restore `DrawnSun::of`'s `if !SkyPhase::sun_is_up(..) { return None; }` and its radiance without the
`twilight_fraction` factor, and restore the two tests' previous claims. The body goes out at the horizon
again, and the night has no sun record.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0956
site: crates/voxel-client/src/sun.rs:324
what: draw the Sun's body at every hour the Sun emits, and fade its radiance by the same twilight share the rig's beam takes
because: the body returned `None` at `0.000°` while `Sky::sun_light` kept the horizon's column alive, so the twilight had light with no source body and the night had no sun record for the Moon's face to reflect
check: cargo test -p voxel-client --lib sun::tests -> failures 0
witness: crates/voxel-client/src/sun.rs
expects: 0
requires: cargo test -p voxel-client --lib renderer::tests::the
requires: cargo check --workspace
```
