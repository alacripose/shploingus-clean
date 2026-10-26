worldgen — the atmosphere at the 1/32 cubic-field scale
======================================================

A **world-generation package**: its rows are the simulation work, and it declares no window and no
layer. The terrain, cloud and sky passes belong to the renderer and the shapers; what a package can
say is *what the simulation must express and at which scale*, and that is exactly what these rows do.

The two sources
---------------

| document | what it contributes |
|---|---|
| [`docs/colorado_atmospheric_effects_scientific_report.pdf`](../../docs/colorado_atmospheric_effects_scientific_report.pdf) | the cautious account: corona vs aureole vs 22° halo, the orographic lift chain, wave clouds, gravity waves and momentum deposition, with nine references. |
| [`docs/deepseek-co-aerosol.md`](../../docs/deepseek-co-aerosol.md) | the summary that asserts a 22° lunar halo and gives the cirrus/prism mechanism, the Colorado cloud depths, and the 20–30 km wave scales. |

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

The output is the package's own statement: name and version, no windows, the three rows with their
words, and `layers none declared`. The reading is kept beside the layer render in
`docs/baselines/mount-v2/`.

What this is not
----------------

* **No physics is implemented here.** The rows are the contract; each one becomes a shaper row (or a
  sky-transport term) with its own measurement, and that measurement is where its acceptance lives.
* **No layer.** A layer is a GPU pass the package brings. World generation runs in
  `voxel-shapers` and the sky's light transport, and adding a pass here would draw something the
  simulation has not computed.
* **No loader.** `Mount::load_display_package` is M10.3; until it lands, this package is source that
  a human reads and a test checks.
