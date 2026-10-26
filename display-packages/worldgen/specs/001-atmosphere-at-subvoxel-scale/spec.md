001 — The atmosphere the world generates, at 1/32 of a voxel
==========================================================

What this is
------------

A world-generation package derived from two documents added on 2026-10-24:

* `docs/colorado_atmospheric_effects_scientific_report.pdf` — seven pages, nine references.
* `docs/deepseek-co-aerosol.md` — a summary of the same photograph and region.

It exists because the world's weather is supposed to be a **cellular automaton running scientifically
backed field shapers over octree terrain**, with a minimum cell of **1/32 of a voxel** — 31.25 mm at
the user's 8 m → 1 m scale (`coord::FILL_SIDE = 32`, `pattern::octave_cell_m(1)`). A mechanism that
only works at a coarser cell is not a mechanism this world can run.

What the documents ask the simulation to express
------------------------------------------------

1. **Optical scattering through thin cloud.** A bright lunar glow can be a **corona/aureole** from
   droplet diffraction, or a **22° halo** from refraction through hexagonal ice crystals, or a
   mixture. The two differ in what controls the angular pattern: droplet size for a corona,
   the crystal's minimum deviation for a halo.
2. **Terrain-forced lift.** Wind approaches rising terrain; the air expands and cools; vapour
   condenses onto aerosols or deposits as ice; the lee side descends, compresses and dries. Cloud
   forms only when moisture, temperature and stability all allow it — the mountains modify an
   existing air mass, they do not create the water.
3. **Mountain waves.** Stable flow over a ridge is displaced vertically, buoyancy restores it, and
   the oscillation propagates upward as a gravity wave that can break and deposit momentum in the
   stratosphere. A wave can exist in a clear sky; wave clouds appear only where the rising branch
   reaches saturation.

**The two documents disagree, and that is part of the requirement.** The PDF's own words: *"a classic
22-degree ice-crystal halo cannot be confirmed from this photo alone"* — its best-fit description is
*"a lunar corona / aureole-like glow through thin cloud or haze"*. The Markdown summary asserts the
halo and explains the prism mechanism in detail. Both mechanisms are real; the identification is not
settled by either document. So the package carries **both**, and what distinguishes them is a
measurement in **angular degrees**, not pixels.

What the documents explicitly do not establish
----------------------------------------------

* The source's identity, cloud altitude, droplet or crystal size distribution, or the photograph's
  date and field of view: **no EXIF was available**, and the report says so.
* That a mountain wave caused this glow. The PDF separates the two phenomena: *"a cloud ring … is an
  optical pattern measured along the observer's line of sight. A mountain wave is a three-dimensional
  airflow disturbance"* — one does not prove the other.
* That a halo predicts precipitation. The Markdown's own forecasting note is a correlation, not a
  law.

Acceptance
----------

The package declares the three rows with the record's words, at the 1/32 scale, and `voxel-layers`
renders them without refusing (`package worldgen v0`, three rows, `layers none declared`). The
**physics** is not accepted here: each row is closed by a shaper (or sky-transport) measurement that
names the cell size it was taken at, and those witnesses are listed as unfinished in `tasks.md`.
