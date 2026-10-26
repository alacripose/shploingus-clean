---
patch_id: PATCH-064
title: The two suns that shared a screen — a measured corner is lit by the rig's own beam
status: applied
operation: fix
target: shploingus/light-units
base_ref: shploingus-clean@main
base_digest: sha256:adbb6a81d5031a04932e34e365b73d10490830ae5bdb4ff415022dee2c14afed
dependencies: [PATCH-063]
source: user-reported-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — many places unexpectedly bright, and the light budget is the request"
live_mutation: false
---

# Change

**The audit, in units, through the real field.** `shading.wgsl`'s fragment adds six things: the rig's
ambient (before the branch), then either the field's six bands or the rig's lights, then the rim, the
coat, the specular step, emission, and — only for a transmissive material — the sky reflection. Every
term is `base ×` an **uploaded** quantity: the rig at `EXPOSURE · grade = 4 · 1` on the day this was
measured, and the field's bands after the same scale (`skylight::exposed_bands`, applied once in
`shploingus`'s `measured_light` closure). `examples/light-terms.rs` transcribes `fs_main`'s sum and
prints it per term, over the shipped world's own streamed window and **the real `Measurer` worker**.

The budget it printed at tick 12000 (sun 22° up, grade 1.0):

| term | measured corner | the same geometry on the rig |
|---|---|---|
| ambient (rig, unconditional) | 0.11037 | 0.11037 |
| beam | **0.91479** (after the fix; **0.14832** before it) | 0.91479 |
| sky (field's stencil only) | 0.01127 | 0.0 |
| rim / coat+spec / emission | 0 (ndotv = 1; this material has none) | 0 |
| total luminance → tone-mapped | 1.03644 → 0.50895 | 1.02516 → 0.50621 |

**So there were two defects, and only one of them was the refusals.** The corner that PATCH-063 fixed
(an unmeasured cell keeping the unshadowed rig) is real and is why the bright patches had mesh edges.
But the measured corner beside it was lit by a beam **5 to 7 times dimmer** than the rig's: the field's
direct band was the band solver's own reading over `peak_sun_luminance`, and the rig's light is
`Sky::sun_light` — a *different model of the same Sun*, at a different absolute scale and with a
different spectrum:

```text
solver, visible bands (B3, B4, B5)   98.1, 131.7, 130.4 W/m²     peak 1091.2
rig sun, rgb (r, g, b)              997.3, 838.9, 542.6 W/m²
```

Per channel the measured beam was `0.153, 0.184, 0.212` of the rig's — not one factor, because the two
models do not share a spectrum. So the frame drew **two suns**: the measured world was dark (tone-mapped
~0.22) and every corner the field had no measurement for drew at ~0.51, which is exactly the reported
"many places unexpectedly bright" seen as islands with hard edges, and exactly the seam the
`AMBIENT_FROM_HORIZON` debt describes from the other side.

**The fix is one law, and the march still decides everything the world can decide.** A march answers
**one transmittance per ray**, not a spectrum, so a cell's direct triple is always *the beam's colour
times that share*. The only open question is whose beam the colour and magnitude are — and it has to be
the rig's, because the two branches are drawn side by side and a corner with nothing in front of it must
be the same light either way. `WorldLight` now carries `beam_rgb` = the rig's first light
(`colour × intensity`, in band order) from the very `Daylight` it was built from, and `assemble`
multiplies it by the marched share. `beam_bands()` still returns the solver's own 12 bands, and the
night's beam is still the solver's `moonlight`, so the *body* and the *direction* remain the solver's;
only the magnitude and colour are the rig's, which is the one thing the solver and the rig cannot both
own without a second sun.

# Acceptance

- **A clear cell reads the rig's triple exactly** —
  `skylight::tests::the_fields_beam_is_the_rigs_own_first_light` now compares magnitude as well as
  direction, on an empty tree where the share is exactly 1.
- **At every hour, including the hours with no light at all** —
  `skylight::tests::the_fields_beam_is_the_rigs_at_every_hour_and_is_zero_where_the_rig_carries_none`
  compares the **exposed** triples (the scale the shader's two branches actually meet at) at noon, an
  eighth after noon, a lit night and a night the rig carries no light in; the dark tick is the empty
  case and both sides are exactly zero.
- **The frame's own sum** — `examples/light-terms.rs` refuses if a measured clear corner is more than
  2 % from what the rig would give it, if a measured shaded corner is brighter than the rig, if a
  tone-mapped value leaves `[0, 1]`, if the worker measures nothing, or if no unobstructed cell exists
  in the window to answer the question at all.
- Nothing about the solver, the stencil, the march, the shader or the exposure changes.

# Evidence

- `cargo run -p voxel-client --example light-terms` — beam ratio **1.000** on the first unobstructed
  cell; measured 1.03644 against the rig's 1.02516; shaded 0.12165 against 1.02516; tone-mapped
  0.50895 against 0.50621.
- `cargo test -p voxel-client --lib` — **534 passed, 0 failed, 2 ignored**.
- `cargo test -p shploingus --bin shploingus` — **2 passed, 0 failed**.

**Named, not fixed — the one term that is still counted twice, with its size.** The fragment adds the
rig's ambient *before* the branch and the field's own sky inside it. On the measured clear corner the
sky is `0.01127` against an ambient of `0.11037` — **10.2 % of the ambient term, 1.1 % of the frame's
luminance**, about one 8-bit step after the tone map, and it is the whole of the 1 % by which the
measured corner sits above the rig's. It is left as it is: the field's sky is the band solver's stencil
in units nothing has reconciled to the rig's two-ray dome, and removing it would be a shader change for
a rounding. The debt is `AMBIENT_FROM_HORIZON`'s, and it is paid where that doc says it is owed — the
sky away from the horizon.

# Rollback

Remove `beam_rgb` from `WorldLight`, restore `out[slot] = (self.beam[*band] * beam_share / self.peak)`,
and delete `examples/light-terms.rs`'s beam comparison. The field returns to the solver's own sun and
the seam returns with it.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0955
site: crates/voxel-client/src/skylight.rs:584
what: multiply the field's direct band by the rig's own per-channel beam, so a measured corner and a rig corner are one light
because: the solver's bands over `peak_sun_luminance` were 0.153/0.184/0.212 of the rig's `colour × intensity` — a measured corner was lit by a 0.15 beam while the corner beside it kept 1.0, which is the bright-edge seam the field exists to remove
check: cargo test -p voxel-client --lib skylight::tests::the_fields_beam -> failures 0
witness: crates/voxel-client/src/skylight.rs
expects: 0
requires: cargo run -p voxel-client --example light-terms
requires: cargo check --workspace
```
