# APPEARANCE — the one area the user owns

> **This document is the user's, not mine.** Every other decision in this repo is
> mine to make; **how the game looks** was reserved by the user in the ruling that
> created this repository. What follows is the *engineering contract* for the
> appearance they specified, and **two interpretations I had to make to write
> code at all**. Both were marked ⚠️ and open to correction — and **both were put
> to the user and confirmed on 2026-09-29** (`LEDGER.md` C-27), so they are no
> longer places I could be wrong about the look. A third question worth more than
> either — where the OpenPBR *numbers* come from — was asked in the same round;
> its answer is §5. The code is in `crates/voxel-appearance`.

## 1. The specified stack

**OpenPBR** (base) **+ NPR** (stylisation), with **4×4×4 subvoxels** and **256×
sub-subvoxels** for block-material detail.

### 1.1 OpenPBR

`OpenPbrMaterial` is the **Academy Software Foundation's OpenPBR Surface**
specification (Andersson, Chen, Dickson, et al., 2024). The subset this repo
implements, and why:

| OpenPBR parameter | Used for |
|---|---|
| `base_color` | albedo, **driven from block truth**, never a per-(kind, direction) constant |
| `base_metalness` | metal vs dielectric split |
| `specular_roughness` | the weathering/erosion signal: eroded rock is rougher |
| `specular_ior` / `specular_weight` | dielectric F0, defaulted per material class |
| `coat_weight` / `coat_roughness` | wet surfaces (hydrology adjacency) |
| `transmission_weight` / `transmission_depth` | water, glass; **never** on solids |
| `subsurface_weight` | soil, snow |
| `emission_color` / `emission_luminance` | dynamic block-emitted light (FC-8.6) |

The twelve old `OpenPbrMaterial` fields were named in the record and had **no
behaviour behind them** (`conformance-census.md:78`: *"OpenPBR appearance — absent
from source; renderer uses fixed per-(kind, direction) colours"*). Here they are
the actual shading inputs.

### 1.2 NPR

The stylisation layer is **Valve's Source 2 shading model** — not three flat
luminance bands. Its four terms, in order:

1. **half-Lambert** diffuse wrap (the `N·L · 0.5 + 0.5` remap),
2. **rim light** (fresnel-shaped, view-facing),
3. **specular warp** (a hard threshold term for the stylised highlight),
4. **ambient tint** by material class.

NPR **never replaces** the OpenPBR BSDF. It is applied *over* it, so that a
non-photoreal look is a **stylisation of correct light transport** rather than a
substitute for it — which is what makes the look tunable without a second
lighting model.

⚠️ **Interpretation 1.** *"NPR" in the old stack was recorded as a criterion, not
an algorithm*: the FMA bars say *"no white-factor unshaded placeholders past S2:
every visible surface shows shading response (**≥3 distinct luminance bands**)"*.
I am treating Source 2 as the **algorithm** that satisfies that **criterion**, and
I will measure the criterion (`≥3 distinct luminance bands` on every visible
surface) as a test.

**Confirmed by the user, 2026-09-29** (`LEDGER.md` C-27). The measurement is
built, and it is tested as a **discriminator** rather than a formality: a
white-factor placeholder measures **1** band and fails the bar, while a stated
OpenPBR + Source 2 surface measures **8 of 8** bands under the declared rig
(`cargo test -p voxel-appearance response:: -- --nocapture`). That the
measurement *saturates* its own resolution is recorded in `STATUS.md` rather than
left to look like a precise figure.

## 2. The two resolutions

| Level | Side | Stored? | Role |
|---|---|---|---|
| **Subvoxel** | 4×4×4 per voxel = **64** | **stored** | *composition*. A voxel's surface material is a **blend over its 64 subvoxels**, weighted by the visible sub-face area. This is what makes a block a shape, not a cube. |
| **Sub-subvoxel (detail)** | **256×** per subvoxel axis | **addressed, not stored** | *detailing*. A deterministic lattice evaluated on demand: `detail(material, subvoxel, i, j, k)`. |

⚠️ **Interpretation 2.** *"256× sub-subvoxels for detailing block materials"* is
read here as a **256× procedural upsampling per axis** of the material detail
field — **addressed, never stored densely.**

**Confirmed by the user, 2026-09-29** (`LEDGER.md` C-27). And the arithmetic that
justifies it, **corrected by measurement** (`LEDGER.md` C-28): `256` is per
**subvoxel** axis, as §2's own table and `DESIGN-SPEC.md` §1.1 both say, so the
dense alternative is

| Level | Dense samples |
|---|---|
| one subvoxel (`256³`) | **16,777,216** |
| one voxel (64 subvoxels) | **1,073,741,824** |
| one chunk (32,768 voxels) | **35,184,372,088,832** — **2⁴⁵**, about **35 TB** at one byte a sample |

A previous draft of this sentence read `16,777,216` as *per voxel* and reached
`5.5 × 10¹¹` per chunk, which is low by exactly the subvoxel count (**64×**). It
is corrected rather than quietly replaced, and a test in `voxel-appearance`
asserts the correct 2⁴⁵ *and* asserts the old figure does not reproduce — the same
shape as §1.2's correction and C-10's. The addressed form costs one hash per
sample and produces the same picture.

**Neither interpretation needed correcting, and this is still the file to correct
one in.** The data model (`voxel-model`) puts both constants in one place
(`SUBVOXEL_SIDE`, `DETAIL_SIDE`), so a correction is a number and not a rewrite —
which is what made it safe to ask the questions late and cheap to have been wrong.

## 5. Where the OpenPBR numbers come from

This document says *which* parameters exist and never what they are, because no
record in this repository contains a value. Asked to choose, the user ruled:

> **Derive from the material's own physics. Refuse the rest.**

The rule the code applies is one sentence: *a parameter is **derived** when the
model's own structure determines it, and **absent** when the world has the value
and the model does not record it.* So `specular_roughness` is the material's own
density read against the admitted set's own range, `transmission_weight` follows
`OpticalClass`, `coat_weight` follows hydrology adjacency, and `base_metalness` is
zero because nothing in the model can declare a conductor. `base_color`,
`specular_ior`, `specular_weight`, `coat_roughness`, `transmission_depth` and
`subsurface_weight` are **named absent** — the first as `Colour::Unadmitted`, the
rest as `None`, and never as zero.

**The consequence is measured and it is the reason this section exists:** an
admitted rock derives 6 of the 12 parameters, so **no admitted material can pass
the bar yet**, because none has an admitted albedo. `STATUS.md` §2 lists the six
as a table of owed content. This is the honest state rather than an omission: the
shading path those six feed is tested on *stated* inputs today, so admitting the
rows changes a table and not an architecture.

## 3. The bars this must satisfy (`fma-bars`)

| Bar | Requirement | How it is checked |
|---|---|---|
| **Deadlock bar 1** | no unshaded placeholders past S2: every visible surface shows **≥3 distinct luminance bands** | `voxel-appearance::shading_response` measures the band count per material |
| **MD1 bar 5** | empty views get a **text note**, never a blank pane | the Mods/resume windows (later) |
| **HIG bullet 3** | motion that conveys a change keeps a **reduced-motion** alternative | shell motion |
| **HIG bullet 4** | text contrast measured **against the actual blurred backdrop** | shell text |
| **Four-bucket rule** | every geometry constant is labelled *MD1 rule / MD1 example / platform translation / project extension* — *never turn a missing value into an invented constant* | `voxel-client::tokens` |

## 4. Materials come from block truth

Appearance is **derived from block information** (the 4×4×4 sub-cube composition)
**plus deterministic variation** — this is the census's own §5 scope clause, and
it is why `voxel-appearance` takes a `VoxelComposition`, not a block id.
