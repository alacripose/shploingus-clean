# Spec 001 — U1 Presentation (appearance → pixels)

- **Unified system:** `U1 — Presentation` (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2)
- **Derived from (documents):** `docs/APPEARANCE.md` §1 (the OpenPBR + Source 2 contract, 4×4×4 and
  256×), `docs/DESIGN-SPEC.md` §1.1 and §2 F13; the FMA bar's *≥3 distinct luminance bands*
  criterion; `LEDGER.md` C-39 (reflectance is sourced) and C-40 (never test the client invisibly).
- **Legacy witness (evidence only):** `crates/voxel-appearance`, `crates/voxel-client`,
  `crates/voxel-client/src/*.wgsl`. Read for *contract*, never as source (Constitution P7).
- **Status:** `specified` · one foundational behaviour implemented and tested (this pass)

## 1. Why this system exists

The player's entire experience of the game is pixels. Everything the other seven unified systems
produce — voxels, materials, terrain, mesh, light — is invisible until this system turns a *surface*
into a *colour*. It is one system, not three, because the appearance contract (OpenPBR inputs), the
stylisation (Source 2 terms) and the presentation (camera + pixels) share one definition of what a
surface is; splitting them is exactly how a shader and a CPU model come to disagree.

It is the first system rebuilt because the governing plan's Lane A orders it first and because its
shaders are the only artefacts in the tree named by *no* record.

## 2. User scenarios

- **US1 (P1) — the world is lit, not flat.** *Value:* a player sees shading response on every visible
  surface. *Independent test:* shade a sweep of surface orientations and count distinct luminance
  bands. *Given* a declared `Surface` and a `Light`, *When* the surface is shaded across its
  orientations, *Then* the result shows **≥ 3 distinct luminance bands** (the FMA bar).
- **US2 (P1) — colour is not invented.** *Value:* a modder trusts that a shown colour came from
  recorded material truth. *Independent test:* a surface whose `Reflectance` is absent derives **no
  colour** and names the absence. *Given* a `Surface` with no admitted reflectance, *When* it is
  shaded, *Then* the result is a named refusal — never `[0,0,0]` and never a default grey.
- **US2 (P2) — an agent measures the same artefact a player sees.** *Value:* a judgement is made on
  the real picture. *Independent test:* the same `shade` call feeds the client and the measurement.
  *Then* the number a judge reads is the number the screen shows. *(Deferred to slice 1's GPU pass;
  the CPU contract is fixed here.)*
- **US3 (P3) — the picture is a pure function.** *Independent test:* shading the same inputs twice,
  and in a different sample order, gives byte-identical output.

## 3. Functional requirements

- **FR-001** `shade(surface, normal, light, view) -> Rgb` **must** apply the four Source 2 terms
  (half-Lambert wrap diffuse, fresnel-shaped rim, specular warp, ambient tint) **over** the OpenPBR
  inputs, never instead of them.
- **FR-002** Inputs — `surface`, `normal`, `light`, `view` — **must be declared types** with no
  implicit global state; the function is pure.
- **FR-003** A parameter the material record does not carry **must** be absent (`Option`/named
  variant), never `0.0`. A `Surface` with no admitted `Reflectance` **must** refuse to derive a
  colour, naming `no_reflectance`.
- **FR-004** A non-normalised `normal` or `view` **must** be refused by name (`not_a_unit_vector`),
  never silently normalised.
- **FR-005** `band_count` **must** mean *the number of distinct luminance bands at a declared
  resolution*, **must** be a lower bound (it saturates at that resolution), and the resolution
  **must** be a single named constant.
- **FR-006** The system **must not** depend on, import or `use` any crate under `crates/`.

## 4. Measurable success criteria

- **SC-001** On the declared sweep fixture, `band_count ≥ 3`. *Decided by:* the unit test
  `the_fma_bar_measures_at_least_three_bands`, which prints the measured count.
- **SC-002** A `Surface` with `reflectance == None` returns `Err(RefusalKind::NoReflectance)`. *Decided
  by:* `a_surface_without_reflectance_is_refused_by_name`.
- **SC-003** `shade` over the same inputs is identical across runs and independent of sample order.
  *Decided by:* `shading_is_a_pure_function_of_its_inputs`.
- **SC-004** The crate's dependency set is exactly what the manifest declares: **one** direct
  dependency (`sha2`, for content identity) plus its transitive crates, and nothing else. *Decided
  by:* `cargo tree` in `clean-room/` lists `sha2` and its transitive crates; `cargo test` runs
  offline.

## 5. Key entities

- **`Surface`** — `reflectance: Option<Reflectance>`, `roughness`, `metalness`. The reduced OpenPBR
  subset the spec names; more parameters are *owed* and named absent, not defaulted.
- **`Reflectance`** — three channel ratios, carried **with a source** (never a bare `[f64;3]`).
- **`Light`** — a unit direction toward the light and a tint.
- **`View`** — a unit direction toward the viewer.
- **`Rgb`** — three linear channels.
- **`BandCount`** — an integer bounded by the declared resolution; `≥` semantics stated in FR-005.

## 6. Edge cases and refusals

| Input | Disposition |
|---|---|
| `reflectance` absent | refuse by name `no_reflectance` (FR-003) |
| `normal`/`view` not unit length | refuse by name `not_a_unit_vector` (FR-004) |
| a channel ratio outside `[0,1]` | refuse by name `reflectance_out_of_range` |
| light behind the surface (`N·L < 0`) | the wrap term carries it (Source 2 half-Lambert); **not** a refusal |
| luminance all-equal (a single band) | a **failed** SC-001, printed as its number — never rounded up to pass |

## 7. Assumptions and open questions

- The appearance *look* — which colours, how the sky reads — **is the user's** (`docs/APPEARANCE.md`
  header). This spec fixes the *mechanism* and leaves the palette to the user.
- The exact OpenPBR parameter count remains open; this slice implements the reduced set the spec
  defines and names the rest absent.
- The GPU pass (a real shader checked against this CPU model) is slice 1's work and is not claimed
  here.
