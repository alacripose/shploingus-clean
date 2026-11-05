biomes — the classified biomes catalog
======================================

The atlas's catalog, as a package the shapers classify from. `docs/earth_biomes_subbiomes_terrain.pdf`
§2, and `docs/earth_biomes_subbiomes_continentalness_mountains.pdf` §5, §15 and §15b, are the source;
this folder is the readable form of them.

What it is
----------

`package.shspec` is the `shploingus.recipe/3` source. Its rows **are** the catalog — one row per
catalog row, in the catalog's order, each row's text a set of `key=value` fields:

| role | fields |
|---|---|
| `envelope` | `name`, `system`, `biotemperature` and `precipitation` (`low..high`, `open` for an unbounded end), `structure`, `biome`, `source` |
| `type` | `name`, `envelope`, `hydrology`, `position`, `structure`, `source` |
| `overlay` | `name`, `hydrology`, `position`, `source` |
| `deferred` | `name`, `waits_on` |

The **envelopes** partition the climate plane: a column's two Holdridge numbers name exactly one arm,
which is the regional default when no local expression applies. The **types** mix an envelope with the
water and terrain position the column actually has, and the **overlays** are §15b's hydrology and
substrate expressions that can sit inside almost any envelope. A **deferred** row is catalogued but
not classified, with the field it waits on — a row whose water and position the ground's laws cannot
produce together, or that a producer cannot tell from an earlier row.

The **order is the classification**: the last matching type wins, so regional defaults come last, and
the first matching overlay wins, so the marsh is before the peat. Do not sort the rows.

`voxel_shapers::biome_library` reads this file: the crate embeds it (`SHIPPED`) so a test or a tool
with no workspace classifies, and `mount` installs a copy read at runtime. The shipped copy and this
folder are one declaration — `the_package_is_the_shipped_source` fails if they are not the same
bytes.

How it is mounted
-----------------

    cargo build -p shploingus

At boot, `crates/shploingus/src/main.rs` reads this folder's `package.shspec`, mounts it, and reports
the catalog it adopted and the climate of the spawn column on the console. The **same binary** with
an edited catalog classifies the spawn column differently — the world's materials, flora and map read
`ecology::biome_for`, which reads the mounted catalog. A package that cannot be read is named once
and the embedded catalog stays: the world is never left without a classifier. The first mount wins
for the process's life, so a catalog cannot change under a classification.

There is **no `package.cells`**: this package declares no window and no drawing, so a cell
declaration would be a line nothing reads. It declares no layer and no frame for the same reason.

Reading it
----------

    cargo run -p voxel-package --bin voxel-layers -- display-packages

Exit 0 means every package folder parsed; this one prints `package biomes v1`, `windows none`,
`rows 50` and each row. The record of those rows is `docs/baselines/biomes.json`, regenerated from
the mounted package:

    cargo run -p voxel-shapers --example biome-sheet-probe -- library

What the rows cannot read yet
-----------------------------

The deferred rows in the same file, with their reasons: salinity, flow accumulation, flood
recurrence, tide, distance to the sea, standing water and groundwater discharge have no producer in
the field plane; the two peat overlays share the marsh's water and flat and wait on a chemistry that
separates them; and the temperate rain forest's wet slope waits on run-off, because saturation and
slope cannot both read (`WET_SLOPE` is below `DRY_SLOPE`).
