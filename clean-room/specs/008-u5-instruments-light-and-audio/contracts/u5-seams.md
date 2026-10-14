# U5's seams — what each surface promises, and what is deliberately not on it

Written after the build (2026-10-12), from the code that exists. Each section names the *record's* own words
its shape comes from, then the six things a consumer may rely on. A promise not on this list is not one this
slice makes.

## 1. `measure` — the instruments (`docs/DESIGN-SPEC.md` §3: S-03, S-12, S-13, FC-2.16)

| Surface | Promise |
|---|---|
| `Sensor` | `site`, `material`, `quantity`, `tick` — and the tick is part of the *value*: `same_moment` tells "two instruments agree" from "two instruments agree because they read the same moment". |
| `Quantity` | `Count(u64)` or `Mass(Mass)`; `count_of()` is `None` for a mass, and that is not an error. |
| `Mass` | One exact rational in grams and one constructor (`Mass::of`). No count, no material name — S-13's own "only raw mass number". |
| `Band` | `min..=max`, both ends inclusive; `Band::of` refuses an inverted band **by name**; `admits` answers `below`/`within`/`above` and nothing else. |
| `GaugeWall` | Many virtual gauges in **one** controller; `watch` refits rather than duplicating; `read` refuses `not_a_count` for a mass (before any lookup) and `unwatched` for an item no gauge answers for. |
| `WeighingGate` | One gate at one boundary; `weigh` returns the exact mass of what crossed, `not_weighable` when nothing did, `unadmitted_material` (the registry's own name) when the density is not there. |

**The vocabulary this adds, closed**: `not_weighable`, `not_a_count`, `band_inverted`, `unwatched`. The crate
declares **90** kinds (*65 when this contract was written — the later passes added the rest: `U7`'s twelve,
`U8`'s three, `U9`'s one, the policy chokepoint's five and the voices' four*) and a test holds
`ALL.len() == COUNT` and every name's round trip.

## 2. `light` — the table and the solver (`docs/light-solver.md`, `docs/missing_atmosphere_deepseek.md`)

| Surface | Promise |
|---|---|
| `Spectra` | Twelve 25 nm bands over 400–700 nm; the Sun's bands **calibrated** to the published `1 361 W/m²`, with that target pinned as a literal in the checks; the Moon's reflectance rising with wavelength; `response` neutral for a flat spectrum; `moon_solid_angle_sr` = `π(R/d)²`. |
| `Air` | Bodhaine's `1.16e-5 m⁻¹` at 550 nm with `λ^-4.09`, and Ångström's aerosol term with the reference's own `α = 1.3` (loading declared, `0.0` a clear column). |
| `Droplets` | `Vis = 0.0219·LWC^-0.9603` km, Koschmieder's `β = 3.912/V`, 99% scattering, and a **layer depth that is a field**: `0.0` is no layer, which is no path. |
| `Solver` | `sun_transmittance`, `direct_beam` (**exactly zero** below the horizon), `sky_radiance` (the closed form plus the night sky's own floor, decayed on the reference's curve), `moonlight` (a reflectance over π, the disc's solid angle, the illuminated fraction, and the air back down). `view_tau` was declared here and is **cut** (2026-10-12): the view column is read by the four readers above and no caller wanted the wrapper — a public method nothing calls is a promise nothing keeps. |

**Not on this surface, and named owed**: ozone, an azimuth (one plane through the Sun), the per-cell frame
format, and `WET_TURBIDITY_GAIN`'s wet-column link. **The one plane is a promise and not an omission**: a
caller with a bearing is asking a question this module does not answer, and says so rather than guessing.

## 3. `audio` — the seam (`docs/LEDGER.md` C-93.5, C-94)

| Surface | Promise |
|---|---|
| `onset(&Noise, tick, at)` | One value per **2 000-tick** window and per **900 m** cell, a function of place and tick and nothing else; the field's own shape declared (rain as a positive anomaly), its scale declared (2.0, for the units), the record's dryness applied. |
| `Weather` | `onset()`, `window()`, `is_raining()` (the *cell's* own floor, 0.22), `is_silent()` / `is_wet()` / `gain()` (the *audio layer's* own reading, 0.05 and 0.18) — two different questions on one value, and the record's own numbers make them different. |
| `duty(&Noise, cells, windows)` | The fraction of samples that read wet — **measured and pinned** to this field's own `0.0482` (a pure function of place and tick is reproducible), reported beside the record's own `0.0997`; C-94's constants are checked as **values** against the ledger's table. |

**Landed since this contract was written (§30–§31):** `feeling.rs`'s wheel read as a value and the **voices** —
one bounded buffer out of the reading's own signature, the weather's own gain and the field's own noise,
with an absent, undeclared, uncalibrated or contradictory reading refused by name. **Not on this surface**:
the **music** and the **ambient beds**, the **material's** own voice and **wind**, and the wheel's own two
source documents, which live outside this tree (§31). C-94 names the seam's first half — *is it raining
right now* — and this is that half.

## 4. The seams U5 does **not** touch

- **U6's module is untouched.** No row, axis, establishment or gate behaviour changed; `admit`'s ten
  arguments and the wire form are exactly as §19–§22 left them.
- **U4's horizons and U3's shapers are untouched.** Nothing here computes a mesh or a stage.
- **U2's registry is read, never extended.** `MaterialRegistry::starter()`'s six materials are the only
  densities the gate uses, and an unadmitted id is a refusal rather than a new row.

## 5. What a consumer may rely on, in one sentence each

1. A band judgement is three words with its band attached — never a score.
2. A gate's answer is a mass and nothing else, exact, and refused by name when it cannot be weighed.
3. A reading no gauge answers for is `unwatched`, not silence.
4. The sky, the beam and the Moon come out of **one** table, and a flat spectrum in it is neutral.
5. No droplet layer is no path — for the beam **and** the sky.
6. A night is dim, and it is not an absence.
