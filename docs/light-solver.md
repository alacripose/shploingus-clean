# The light solver, and the reasons its constants are what they are

`voxel-light`'s prose budget is a ratchet (11% of the crate's bytes may be plain comment, and the
share may fall but not rise — `conformance/prose-frame.txt`). The derivations that were written in
`solver.rs`'s comment lines live here instead, because the gate's own rule is that the explanation a
crate needs belongs in a document a reader looks in, not in a code artifact: *"Move the explanation
rather than arguing the number up."* The code keeps one line per decision and points here.

Nothing in this document is new arithmetic. Every number below is what the crate computes, measured
by its own tests, and each claim names the test that holds it.

## 1. The sky is the single-scatter integral, in closed form

For one band, along one ray, the scattered radiance is

```
L = E_sun,band · (β_s/β_e) · (1 − t_view) · P(θ) · T_sun + floor
```

* `β_e` is the band's extinction per metre plus the droplets'; `β_s` is the band's scattering per
  metre plus the droplets' — one medium, two coefficients. The droplets scatter 99% of what they
  take (`DROPLET_SINGLE_SCATTER_ALBEDO`), which is why fog *loads* a sky as it hides the ground
  behind it: the light is not lost, it is moved into the line of sight
  (`more_droplets_put_more_light_into_the_sky_and_take_more_out_of_the_path`).
* `(β_s/β_e)·(1 − t_view)` is the single-scatter integral solved rather than marched: the share of
  the band the medium scatters, times the share of the path it has not transmitted away. It is the
  one place the air's geometry and the droplets' are merged rather than kept apart.
* `t_view = exp(−τ_view)`, with `τ_view = β_air·(scale_height · m_view) + β_drop·(droplets' own
  crossing)` — **two media, two geometries**. The air is a scale height deep; the droplets are their
  own layer tens of metres deep, so a ray looking up leaves the droplet layer quickly while it is
  still inside the air. One shared column would extinguish the beam before it arrived and the fog
  would never brighten anything.
* `m_view = 1/sin(elevation)`, floored at `GRAZING_AIR_MASS = 38`, so a ray along the horizon is a
  long path rather than an infinite one.
* `T_sun` is the sun's own transmittance through the *same* column, by Kasten & Young (1989)'s air
  mass — `air_mass`, the formula `voxel_shapers::sky::air_mass` also carries, duplicated because this
  crate links nothing; a client holding both can compare them. Below the horizon the formula leaves
  its domain and returns the horizon's own value, with `twilight_fraction` supplying the decay: a
  depressed sun's light does not cross a *longer* path, it crosses the same horizon path in a sky
  whose lit air is shrinking.
* `P(θ) = mixed_phase(...)`: Rayleigh's `3/(16π)·(1+cos²θ)` and a Henyey–Greenstein at
  `DROPLET_ASYMMETRY_G = 0.85`, weighted by the two terms' own optical depths. The droplets' forward
  lobe is what makes a fog's sky bright *around the sun*, which a Rayleigh-only phase cannot produce.
* `floor` is a **radiance**, added and not scaled by the band's power: the sky at astronomical
  twilight is the night sky's own measured value, `NIGHT_SKY_ILLUMINANCE_LX = 0.001 lx` divided by
  683 lm/W and by π (`night_sky_radiance_w_m2_sr`). That is the user's own reference figure
  (`docs/missing_atmosphere_deepseek.md` §4.5), not a floor chosen so the picture is not black.

`the_daylight_sky_is_blue_for_rayleighs_own_reason` pins the consequence: the blue comes out of
β(λ) falling as λ⁻⁴·⁰⁹, and nobody picked it.

## 2. The twilight, and why a night is dim rather than absent

Two curves, and the visual result is entirely the difference between them
(`the_twilight_falls_smoothly_and_the_beam_does_not`):

* the **beam** (`direct_beam`) is exactly zero the moment the planet occludes the body. Not "small":
  zero. A beam dimmed by an enormous factor would be a lie about where the light went.
* the **sky** keeps falling for eighteen degrees after the sun sets, on the reference's own anchored
  exponential `E = 400 lx · exp(0.85·alt)` (`TWILIGHT_DECAY_PER_DEG`, §4.2 of the same reference),
  and lands on the night sky's own radiance at `ASTRONOMICAL_TWILIGHT_DEG = −18`.

`the_twilight_is_the_references_own_curve` reproduces the reference's own table to its rounding:
400 lx at 0°, 13.35 at −4° (the table prints "~15"), 2.40 at −6° (the table prints "~2.5"). The
**formula** is implemented and the two are within the table's rounding — said out loud in the test
rather than hidden by a loose bound.

The defect this whole module exists to end is recorded in `a_night_is_never_black`: the old model
returned zero below the horizon, the rig's ambient followed it to zero, and the world stopped
drawing its materials at sunset. A night is dim; it is not an absence.

## 3. The Moon is a lit surface, and this is the arithmetic of it

The Moon is **not an emitter**. Per band, its contribution is

```
L_moon  = E_sun,band · reflectance_band / π          (Lambert's law)
E_moon  = L_moon · Ω_disc · illuminated_fraction     (the disc's own solid angle)
delivered = E_moon · exp(−τ)                          (the air, on the way back down)
```

where `reflectance_band` is `Band::moon_reflectance` — a reflectance the band table already declares,
rising with wavelength — and

```
Ω_disc = π · (R_moon / d_moon)²     (MOON_SOLID_ANGLE_SR)
```

**The π is the whole of a real discrepancy, and it is recorded rather than papered over.**
`voxel_shapers::sky`'s `SOLID_ANGLE_DILUTION` is `(R/d)²` alone — the *fraction of the sky* the disc
covers, not the solid angle it subtends — and the shapers multiply a radiance by it as though it were
an angle in steradians. Their full moon is therefore a factor of **π too dim**, and their own test
pins their own derivation, so it passes while it is wrong. This solver uses `π(R/d)²`. The two
disagree by π and that disagreement is a real, open defect in `voxel_shapers::sky`.

What the numbers come to, measured on the shipped band table at the zenith with a clear column
(`the_moon_is_a_reflectance_and_not_an_emitter`, `the_full_moons_illuminance_lands_where_the_published_figures_put_it`):

| quantity | value |
|---|---|
| full moon overhead, twelve bands | `3.0484e-3 W/m²` |
| at the 683 lm/W *photopic* conversion | `2.082 lx` |
| the same, at the Moon's own ≈100 lm/W | **≈0.3 lx** — the published full-moon figure |
| sun ÷ moon | `4.5e5`, pinned as six orders |

The 683 lm/W figure is a *warning* printed beside the measurement, not the answer: the Moon's
spectrum is reflected sunlight, whose lumens-per-watt is nearer 100 than 683, for the same reason the
crate measures it coming out redder than the Sun (its reflectance rises with wavelength).

The tests assert what an emitter cannot satisfy:

* **doubling the Sun doubles the moonlight** (an independent source would not move);
* **a tenth-lit crescent is exactly a tenth** of the full disc, to `1e-12` — a fraction of the disc,
  not a dimmer Moon;
* **an occluded Moon is exactly zero**, the same rule the Sun's own beam obeys.

## 4. The response: why a band's colour is an integral and not a sample

`rgb_response` is Wyman, Sloan & Shirley (2013)'s single-lobe fit — a cited approximation, not a
taste. Two calibrations are load-bearing:

* **A band is a range of wavelengths, so its colour is an integral over the range**, on a log grid
  with `nm` as the Jacobian (the same quadrature `spectra` uses for Planck), not the response sampled
  at the band's centre. The centre-sampling version under a sqrt-of-width weight was my arithmetic
  rather than the model's, and it read as a green sky at a low sun — which is how it was caught.
* **Each lobe is divided by its own integral over the visible range** (`response_denominator`): the
  published lobes are not balanced against each other (blue's head is 1.258 against red's 1.065 and
  it is narrower), so equal power in every band came out blue until each channel was normalised.
  Without it the picture carries a cast — a bias in the instrument, not in the world
  (`the_response_makes_a_flat_spectrum_neutral`).

The sun's own bands render near neutral (`(0.33, 0.33, 0.34)`-ish, printed by
`the_suns_own_spectrum_comes_out_near_white`) because it is a 5 778 K body and this model's reference
white.

## 5. The droplet optics, and where they come from

`droplet_extinction_per_m` implements `Vis = 0.0219 · LWC^-0.9603` (km, for g/m³) from
`docs/fog_mist_foam_and_aerosol_dispersion.pdf`, turned into an extinction by Koschmieder's
`V = 3.912/β`. So **the droplets decide the visibility** rather than a humidity reading being turned
into one. At the declared fog threshold `0.021 g/m³` the relation gives about a kilometre, and a
kilometre at that load is 3.912 optical depths (about 2% transmitted) — which is exactly why that
threshold is `0.021` and not a rounder number
(`the_references_own_visibility_relation_is_the_one_used`).

A cloud's extinction (`cloud_extinction_per_m`) is the column's water path spread over one scale
height and put through the *same* droplet optics, so a cloud and a fog are one optical model read
twice rather than two models.

**One medium, two geometries — and the beam was the exception until this pass.** The air is a scale
height deep; the droplets are a layer of their own depth, so each is charged over its own path
(`path_in_layer`). `sky_radiance`, `moon_radiance` and `transmittance` all did that; `direct_beam`
carried the droplet rate over the whole column instead, which made a cloud extinguish the Sun's beam
far harder than it dims the sky it hides, and made the day's beam and the night's — the same beam,
reflected — disagree about the same air. It was found by measuring the field's own day:night ratio,
which came out `inf` because the day's beam under the same air was *exactly* zero.
`the_beams_droplets_are_their_own_layer_and_not_the_whole_column` holds the fix: no droplet layer
declared is no path (the beam is untouched), a deeper layer costs more, and a higher Sun crosses less
of it.

## 6. Where the bands are read now, and what is still owed

**The bands reach the world, per cell.** `voxel-client` reads this crate for the rig's own lights
(`SUN_DILUTION`, and the Moon's reflected bands) and for the world's own light field:
`skylight::WorldLight` is one measurement per cell whose beam is `direct_beam` while the Sun is up and
`moonlight` while it is not — the same two calls the rig's first light is built from, so doubling the
Sun doubles both — and whose direction is that same body's, held against `rig.lights[0].direction` by
`the_fields_beam_is_the_rigs_own_first_light` (a shader wraps the field at that light's angle, so the
radiance and the angle may not come from different bodies).

What reaches a cell is the transmittance **marched through the world's own octree**:
`voxel_stream::light::transmittance_tree` walks `SparseVoxelOctree::spans` (`voxel-mesh`), which
descends the hierarchy rather than stepping the grid — an empty metre costs one slab test and a solid
block one charge rather than one per voxel. The two walks are held to each other at `1e-12` over a
battery of rays through a generated chunk (`the_tree_walk_and_the_voxel_walk_are_one_measurement`),
and their costs are measured rather than asserted: a beam out of the spawn chunk's sky is 41 voxels
stepped against 0 spans descended. At a lit night a cell in the open reads the Moon's beam and a cell
three metres into the rock reads `[0, 0, 0]` (`the_nights_beam_is_the_suns_reflected_and_the_octree_
says_how_much_arrives`) — which is the shadow the rig cannot cast, because the rig does not know where
a cell is.

**The frame reads it, and the format it reads it in is the remaining step.** `shploingus`'s frame
takes the field on the atmosphere's cadence (`main.rs`'s `CellField::of`) and builds the scene with
`Scene::build_with_light` whenever it is present — the per-cell measurement reaches the vertex format
and `shading.wgsl` shades from it as `light_present` — and falls back to `Scene::build` and the rig
only when there is no field at all. What is still owed is the **format**: the game's own cheap path,
the quad format (C-168: 24 camera-independent bytes and no measurement), cannot carry a per-cell
value, so a frame built from records is lit by the rig. That is a renderer change rather than a solver
one, and it is the octree-aligned channel the light work is for.

**And one smaller one:**

* **The π** in §3, in `voxel_shapers::sky`, which its own test cannot see.
