worldgen — the atmosphere and the world's own constants
========================================================

A **world-generation package**: it declares no window and no layer, and holds two kinds of row.
The terrain, cloud and sky passes belong to the renderer and the shapers; what a package can say is
*what the simulation must express, at which scale, and with which numbers*, and that is what these
rows do.

* `OPT`, `ORO`, `WAV` — the atmosphere and optics the two Colorado documents require, in prose.
* `GEN-0001` … `GEN-0007` — **the world's constants**: the continental wavelength, the relief, the
  sea line, the datum, the equatorial base temperature and the two ridge terms. `voxel_shapers::worldgen`
  reads these rows into the `Config` every `WorldSpec` is built from, so the depth of the sea and the
  wavelength of a continent are authored here rather than compiled into `spec.rs`. The game mounts
  this folder at boot (the same act as the biomes catalog) and prints the constants and the spawn
  column's elevation; an edited row changes the ground the same binary draws.

The two sources
---------------

| document | what it contributes |
|---|---|
| [`docs/colorado_atmospheric_effects_scientific_report.pdf`](../../docs/colorado_atmospheric_effects_scientific_report.pdf) | the cautious account: corona vs aureole vs 22° halo, the orographic lift chain, wave clouds, gravity waves and momentum deposition, with nine references. |
| [`docs/deepseek-co-aerosol.md`](../../docs/deepseek-co-aerosol.md) | the summary that asserts a 22° lunar halo and gives the cirrus/prism mechanism, the Colorado cloud depths, and the 20–30 km wave scales. |
| [`docs/earth_biomes_subbiomes_continentalness_mountains.pdf`](../../docs/earth_biomes_subbiomes_continentalness_mountains.pdf) | the world constants' meaning: §1 the climate engine, §2 continentalness, §3 mountains and uplift. |
| [`docs/earth_biomes_subbiomes_terrain.pdf`](../../docs/earth_biomes_subbiomes_terrain.pdf) | §1 the first filter (energy and water), §3 terrain as a climate modifier, §7 mountains and topographic diversity. |

**The two disagree about the photograph, and this package keeps the disagreement.** The PDF says a
classic 22° halo *cannot be confirmed* from the image; the Markdown says the ring *is* a 22° halo. So
`OPT-0001` carries **both** mechanisms — droplet diffraction and ice-crystal refraction — and the
identification is left to a measurement in angular degrees. A package that picked one would be
inventing an observation, which is the failure this tree's refusals exist to prevent.

The scale
---------

Every row runs at the **1/32 cubic-field shaper scale**: `coord::FILL_SIDE = 32`, and
`pattern::octave_cell_m(1) = 31.25 mm` at a 1 m voxel (`8 m → 1 m`, the user's blending tiers). A
mechanism that cannot be sampled at that floor is named absent rather than approximated at a coarser
cell, and the rungs **blend** on the terrain's own ladder (M2.0/M2.1).

Read it
-------

    cargo run -p voxel-package --bin voxel-layers -- display-packages/worldgen

The output is the package's own statement: name and version, no windows, all ten rows with their
words, and `layers none declared`. The mount itself and its measured elevations are recorded in
[`docs/baselines/worldgen.json`](../../docs/baselines/worldgen.json).

What this is not
----------------

* **No physics is implemented here.** The rows are the constants and the contract; the laws that
  read them live in `voxel-shapers`, and each row's acceptance is the measurement beside it.
* **No layer.** A layer is a GPU pass the package brings. World generation runs in
  `voxel-shapers` and the sky's light transport, and adding a pass here would draw something the
  simulation has not computed.
* **No second loader.** `voxel_shapers::worldgen::mount` is the world's loader, the same act as the
  biomes catalog's; the shell's `Mount` is for packages that declare windows and layers, which this
  one deliberately does not.
