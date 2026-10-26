001 — Plan: where each mechanism runs, and at which cell
=======================================================

The rule this plan is written under: **a mechanism that cannot be sampled at the 1/32 cubic-field
floor is named absent, not approximated at a coarser cell.** The rows in `package.shspec` are the
contract; the table below says who implements each one and what measurement closes it.

The three rows
--------------

| row | mechanism, from the documents | where it runs | what closes it |
|---|---|---|---|
| `OPT-0001` | droplet diffraction (corona/aureole, droplet-size controlled) and ice-crystal refraction (22° minimum deviation) | the sky's light transport — `voxel-light::spectra`, `voxel-client::atmosphere`, `skylight` | an angular measurement: the ring's radius in degrees at a declared droplet/crystal size distribution, at a 31.25 mm cell, against the 22° minimum-deviation law |
| `ORO-0001` | terrain-forced lift: approach, expansion and cooling, condensation, lee descent | `voxel-shapers::climate` (lift and moisture), `voxel-shapers::clouds` (condensation and the cloud field) | a parcel path over a declared ridge: saturation height and cloud base, with the lee side drying; the same numbers at a 31.25 mm cell and at a 32 m cell |
| `WAV-0001` | mountain waves: displacement, buoyancy oscillation, upward propagation, breaking, momentum deposition | the upper-atmosphere field; no module today | a displacement field over the same ridge whose vertical wavelength is declared and whose momentum deposit is reported, **or the row stays open** |

**None of the three is implemented by this package.** It is a declaration: three rows with the
documents' own mechanisms and the scale they must run at. The table's "where it runs" names the
module that would own the work, and "what closes it" is the witness that would let the row be
ticked. A row with no module is written as *no module today* rather than pointed at the nearest
plausible one.

The scale, stated exactly
-------------------------

* `coord::FILL_SIDE = 32` — a voxel's edge is divided into 32 cells, so the field floor is
  `1/32` of a voxel.
* `pattern::octave_cell_m(1) = 31.25 mm` at the user's `8 m → 1 m` scale — the smallest cell the
  material ladder can address, and the reason a mechanism is refused rather than coarsened.
* The rungs **blend** on the terrain's ladder (`pattern::octave_weight`, M2.1), so a cloud or a
  lift field sampled at 31.25 mm does not switch to a coarse rung at a threshold: its weight moves,
  and the frame's seam is the witness.
* The two documents' own scales are recorded as **context, not as constants**: cloud depths of
  4,000–9,000 ft and bases near 12,000 ft MSL (the Markdown), wave lengths of 20–30 km (the
  Markdown) and the 8–15 km tropopause (the PDF). None of them is a cell size; they are the
  phenomena's scales, and the simulation's cell is 31.25 mm until a measurement says otherwise.

What the photograph does and does not constrain
-----------------------------------------------

The PDF's evidence boundary is carried verbatim into the package: no EXIF, no angular calibration, no
sounding or satellite scene matched to the image. So `OPT-0001` carries **two** mechanisms and does
not choose. A renderer that drew a 22° ring because the Markdown said so would be asserting an
observation the record calls unconfirmed; a renderer that drew a corona because the PDF preferred it
would be doing the same in the other direction. What the package requires is that **both** be
expressible and that the measurement that distinguishes them be angular.

Order of work
-------------

1. The declaration (this package): rows, scale, sources, and the disagreement recorded.
2. `ORO-0001` first — the lift chain is the mechanism with an existing home (`climate`, `clouds`)
   and the one whose cell size can be measured today.
3. `OPT-0001` second — the sky transport already carries spectra; the missing piece is a scattering
   term with a declared angular law, not a new sky.
4. `WAV-0001` last, and **only with a module** — an upper-atmosphere field that does not exist yet.
   Until then the row is open, and the package says so rather than drawing a wave.

Not in scope
------------

* The look of any of the three (colour, bloom, exposure): those are the user's.
* A GPU layer for the simulation: world generation runs in the shapers and the sky's transport, and
  a layer here would draw something no shaper computed.
* The loader (`Mount::load_display_package`, M10.3) and any pixel of the final frame.
