---
patch_id: PATCH-066
title: The night that opened on a new moon — the shipped world's lunar epoch moves half a month, so the default night has a lit disc and a beam
status: applied
operation: fix
target: shploingus/lunar-epoch
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-064, PATCH-065]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the default night must begin at a full moon, so the drawn body and the field's beam have a Moon to carry; the sun body and the rig's own colour are already in place (PATCH-064, PATCH-065)"
live_mutation: false
---

# Change

**The shipped run opened at a new moon, and nothing about the night could be seen for it.**
`moon_phase_fraction` is `(1 − cos E)/2` on `moon_elongation_deg`, and that elongation is measured from
a **new-moon tick** the caller supplies — a property of the world, not of the mapping. Every call site in
`shploingus` passed **`0`**, and at the world's own start tick `GAME_TICK = 9000` that is `0.375` days into
the cycle: an elongation of **`4.57°`** and a phase of **`0.0016`**. The first night therefore had no lit
disc for `DrawnMoon::of`, no reflected beam for `moonlight`, and **no light in the rig at all** — the
remaining half of the reported night, after the sun body (PATCH-065) and the rig's own colour (PATCH-064).

**What moved is the epoch, not the map and not the sun.** `GAME_NEW_MOON_TICK` is declared as
`GAME_TICK + HALF_SYNODIC_TICKS`: half a synodic month *after* the opening tick, which puts the opening at
an elongation of `−180°` — a **full moon** — waning from there over the next fortnight. The phase
mapping is untouched, and `GAME_TICK` is untouched, because it was chosen for the *sun* (`10.57°`,
long shadows, night a quarter-day away) and moving it would move the sun and the clock together to reach a
Moon.

**An epoch after the tick needed the arithmetic fixed, and that is the second half of this record.**
`mean_longitude_deg` subtracted with `tick.wrapping_sub(epoch)`: an epoch ahead of the tick read as
`1.8e19` ticks, an angle of `2.4e21°`, and a cosine of noise. It is now a **signed** difference
(`tick as i128 - epoch as i128`), so a world that opens before its next new moon has a negative
elongation — a Moon still behind the Sun, which is what a waning body is — and the two sides of the epoch
are one continuous function (`an_epoch_ahead_of_the_tick_is_a_waning_moon_and_not_an_overflow`).

The epoch is threaded through every derivation the frame makes: the boot `Daylight`, the boot sky model,
the boot bodies, the cadence's `daylight_of`/`WorldLight`/`sky_model_at`/`bodies_at`, and the look-ahead's
`Ask`. One value, one place (`game.rs`), so the rig, the sky, the drawn bodies and the field's beam cannot
describe two different Moons.

# Acceptance

- **The default night opens on a full moon** —
  `skylight::tests::the_shipped_night_opens_on_a_full_moon_and_the_ground_receives_its_light` walks the
  shipped world through `moon_position` → `DrawnMoon::of` → `daylight_of` → `WorldLight::with_daylight`:
  opening tick `9000` is phase `1.0000`, elongation `−180.0°` (and `0.0016` at the old epoch, which the
  test asserts so it is measuring the defect it names); the first night with the Moon up is tick `21 337`
  (Sun `−50.9°`, Moon `10.0°`, phase `0.997`) with a drawn radiance of `1.51e-1 W/m²/sr`, one rig light at
  intensity `2.10e-6`, a beam luminance of `1.90e-6`, and the field's own bands equal to the rig's beam
  channel for channel. **The night is not a day**: the beam is `4.03e-6` of the opening Sun's, and the
  rig's ambient (`1.37e-8`) is more than four orders under the day's (`7.5e-2`).
- **The frame's exposure is named, not conflated with the physics.** The same test prints the graded
  ground: `grade 4.76e5` lifts the night's beam to `3.61` against the day's `1.54` — `exposure_grade`'s
  declared job is to expose for the Moon it has, and it is unchanged here. What stays night is the sky
  (`2.6e-2` ambient against the day's `0.896`), no Sun, and a grey light at a walking-pace angle.
- **An epoch ahead of the tick is a state, not an overflow** —
  `sky::tests::an_epoch_ahead_of_the_tick_is_a_waning_moon_and_not_an_overflow`: half a month ahead is
  `−180.000°` and phase `1.000000`; a lead of `1000` ticks is negative and a lag is positive, and the two
  sum to zero within `1e-9`, so the elongation crosses the epoch continuously.
- **No existing assertion moved on this record's own tree — and the claim is scoped there.** `cargo
  test -p voxel-client --lib` was **535 passed, 0 failed, 2 ignored** there (534 before this test was
  added; 536 once PATCH-067 adds its twilight test); `cargo test -p voxel-shapers` was **362 passed,
  0 failed, 1 ignored** (361 before); `cargo check --workspace` finished with no errors. **Later work
  moved some, and this record should not be read as saying otherwise:** PATCH-068 changed this
  record's own night test (it now prints the measured moon-to-sun ratio beside the drawn one),
  repaired two of the orbit's tests, and replaced the exposure; the crate reads **543 passed, 0 failed,
  2 ignored** today. The opening-phase assertions this record added are unchanged.

# What this does not touch

- **The sky double-count** named open in PATCH-063: untouched.
- **Shadow sampling**, the tree mob (PATCH-043/057), the gate, and the walk: untouched.
- **The exposure grade**: untouched, and its tension with "a night that draws darker than a day" is
  printed in the test rather than tuned away here.

# Evidence

- `cargo test -p voxel-client --lib the_shipped_night_opens_on_a_full_moon -- --nocapture` — **1 passed**,
  printing: `opening tick 9000: phase 1.0000 (was 0.0016 with the epoch at 0), elongation −180.0°`;
  `first night tick 21337: sun −50.9°, moon 10.0° up, phase 0.997`; `drawn moon radiance 1.512e-1
  W/m²/sr`; `rig: 1 light(s), intensity 2.100e-6, beam luminance 1.897e-6, ambient luminance 1.367e-8`;
  `moon-to-sun beam at the ground: 4.03e-6 (day intensity 0.5213)`; `grade 4.762e5: exposed ground 3.613
  beam + 2.605e-2 ambient, against the day's 1.544 + 0.896`.
- `cargo test -p voxel-client --lib` — **535 passed, 0 failed, 2 ignored**.
- `cargo test -p voxel-shapers` — **362 passed, 0 failed, 1 ignored**.
- `cargo check --workspace` — **Finished**, no errors.

# Rollback

Restore the epoch argument at the six `shploingus` call sites to `0`, delete `GAME_NEW_MOON_TICK`, and
restore `mean_longitude_deg`'s `tick.wrapping_sub(epoch)`. The world opens at phase `0.0016` again, the
first night has no Moon, and an epoch ahead of the tick is noise again.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0957
site: crates/voxel-client/src/game.rs:152
what: declare the shipped world's lunar epoch half a synodic month after its start tick, so the opening is a full moon, and thread it through every sky derivation
because: the run passed `0`, so tick 9000 sat at elongation 4.57° and phase 0.0016 — a new moon whose night had no lit disc, no reflected beam and no light in the rig, and `wrapping_sub` read the new epoch as an overflow
check: cargo test -p voxel-client --lib the_shipped_night_opens_on_a_full_moon -> failures 0
witness: crates/voxel-client/src/skylight.rs
expects: 0
requires: cargo test -p voxel-shapers
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace
```
