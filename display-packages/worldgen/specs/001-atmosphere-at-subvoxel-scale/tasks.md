001 — Tasks: what is declared, and what is owed
===============================================

Declared (this package)
-----------------------

- [x] `OPT-0001` — both optical mechanisms the documents name, as scattering terms of one sky
      transport, distinguished by an angular measurement rather than a pixel radius.
- [x] `ORO-0001` — the terrain-forced lift chain, with cloud conditional on moisture, temperature and
      stability; a rain shadow as an outcome, not a guarantee.
- [x] `WAV-0001` — terrain-generated gravity waves through propagation, breaking and momentum
      deposition, with a wave allowed to exist in a clear sky.
- [x] Every row carries the **1/32 cubic-field shaper scale** (`coord::FILL_SIDE = 32`,
      `pattern::octave_cell_m(1) = 31.25 mm`) in its own words.
- [x] The two documents are cited, and their **disagreement about the photograph** is recorded in
      `spec.md` and in `package.shspec` rather than resolved by fiat.
- [x] The evidence boundary is carried: no EXIF, no angular calibration, no sounding matched to the
      image; the PDF says a classic 22° halo *cannot be confirmed* from it.

Owed (the rows' acceptance)
---------------------------

- [ ] `ORO-0001` — a shaper measurement of a parcel path over a declared ridge: saturation height,
      cloud base, and lee-side drying, at a 31.25 mm cell **and** at a 32 m cell. Witness:
      `docs/baselines/` under the pass that takes it.
- [ ] `OPT-0001` — a scattering term with a declared angular law (droplet diffraction for the corona,
      the 22° minimum deviation for the halo) and a ring radius printed in **degrees** at two
      declared size distributions. The term belongs in the one sky transport; a second sky is
      refused (`C-33`).
- [ ] `WAV-0001` — **no module today.** An upper-atmosphere displacement field with a declared
      vertical wavelength and a momentum-deposit report. Until it exists the row is open, and no
      layer, shader or frame may pretend it ran.
- [ ] The package's own `claims.shpkg` (M10.3), as for every display package.

How the declared tasks are checked
----------------------------------

    cargo run -p voxel-package --bin voxel-layers -- display-packages/worldgen

Exit 0, with `package worldgen v0`, the three rows in full and `layers none declared`. The owed
tasks have **no check here on purpose**: a check that cannot run is not evidence, and each one names
the measurement that would make it real.
