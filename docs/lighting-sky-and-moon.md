# the lighting, the sky, and the Moon's transport — the long form

**Status: `DocumentRole::Derived` — a reference, not authority.** The record is `docs/LEDGER.md`;
where this file and the record disagree, the record wins and this file is wrong. An anchor that
exists only here may not be cited by a feature: the build gate refuses it, exactly as it refuses one
that exists only in `docs/HANDOFF.md`.

**Why these pages exist.** `VOWS.md` caps an in-code docstring at one or two sentences, and the prose
ratchet (`conformance/prose-frame.txt`, `census::check_prose_frame`) caps each crate's share of
*plain* comment — the `//` lines a reader can only find by standing in the file — at the number it
measured on the day the ratchet was switched on. Rustdoc is not counted; plain comment is. The gate's
own refusal states the remedy in those words: *"the explanation this crate needs belongs in a
document under `docs/`, where a reader looks for it, and prose in a code artifact is documentation
written where nobody reads it. Move the explanation rather than arguing the number up."* This file is
that move, for the light.

**Every section below is moved text, not a paraphrase.** Each one is the comment block exactly as it
stood, with its `//` markers off and its own `#` headings demoted so they nest here; nothing was
reworded, shortened or merged, so no fact was lost. The footer under each section names the file and
the lines it stood in, and the source now points back here from that place.

**What stayed in the code, and why.** Two kinds of prose did not move. A *field's* own line or two --
the sentence beside the member it describes, in `shading.wgsl`'s `View` and `sky.wgsl`'s `SkyView` —
is exactly what `VOWS.md` asks a comment to be, and a reader of a struct should not
have to open another file to learn what a member is for. And two long blocks in `shading.wgsl` are not
about light at all: the vertex format (`QuadRecord`, and the expansion of one record into two
triangles) and the debug wireframe (F4), because a reader looking for the record format should not
have to open a page about the sky.


## from `crates/voxel-client/src/shading.wgsl`

*moved from lines: 1-14, 193-200, 321-328, 331-344, 435-463, 634-644, 692-701, 850-870, 922-933, 935-948, 986-994, 1028-1037, 1047-1064, 1094-1101, 1115-1123, 1195-1217*

### the Valve Source 2 stylisation, transcribed into WGSL

The Valve Source 2 stylisation, as `voxel-appearance`'s `npr::shade` states
it -- written a second time, in WGSL, because a shader cannot call Rust.

This file is a *transcription*, and it is held to the original by
`renderer::tests::the_gpu_shades_a_surface_exactly_as_the_crate_does`, which
renders stated normals and compares the pixels against `npr::shade`. The
branch order, the exact expressions, and the `_present` flags are all copied
deliberately: a "nicer" formulation here would be a second model, and two
models is how shading drifts.

The one line that must NOT be simplified: `if (base_present > 0.5)`. An
absent albedo removes the diffuse lobe. Reading it as black instead would
change `Terms`, and `Terms` is what the FMA bar's band count is reported
against.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1-14 (14 lines).*

### The cloud's veil

**The cloud's veil** (C-178/C-179): this cell's own column transmittance, from
`cloud_cubes` through the origin's `w` — **signed by which of the two trees a fade is
joining the cell belongs to** (C-183, `cloud_cubes::veil_with_side`). Zero for every
surface, which is terrain: the world never wrote that float before clouds existed, and
`0` is the one value a cloud cannot carry, so "no cloud" stays an exact test.

Flat, for the reason `light_present` is: it is a property of the cell and not a
quantity to blend, and every corner of one cell carries the same one anyway.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 193-200 (8 lines).*

### The fragment reads the composition here, not the quad's own material id

The fragment reads the composition here, not the quad's own material id.
The quad's own material is the selected one, so for a single-material quad
this is the same id the fragment shades from; for a genuinely mixed selection
The fragment reads the composition here, not the quad's own material id.
The quad's own material is the selected one, so for a single-material quad
this is the same id the fragment shades from; for a genuinely mixed selection
the composition is the face blend, and the shader blends the selected materials
instead of looking up one id's row.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 321-328 (8 lines).*

### No per-vertex lattice sample and no per-vertex light

**No per-vertex lattice sample and no per-vertex light.** A record has no
room for either. `light_present = 0` is not "a field of zero here" but
"this format carries no field", which is the rig path — what every frame
was lit by before a field existed.

**`detail_roughness`/`detail_reflectance` are zero here, and `sample_present`
is what says so.** They used to be justified with "neither is lost: the
fragment's own ladder is the whole address law for this geometry", and that
was **false at distance**: the ladder is deliberately silent past its ceiling
(C-163), and the fragment's blend falls back to the corner sample exactly
there — so a corner of zero meant flat albedo past the horizon, which is the
look the user reported losing on 2026-10-10. The fragment now reads the
Sun's own angle wherever this flag says the corner has no sample
(`lod_reflectance_delta`), and the zero below is left as what it is.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 331-344 (14 lines).*

### the pattern's own arithmetic (C-161, C-162, laddered by C-163)

---- the pattern's own arithmetic (C-161, C-162, laddered by C-163) ----------

A material's 256-level detail is **addressed, never stored** (C-2), so a
greedy-merged rectangle can still show its blocks: the fragment's own **world
subvoxel** addresses the lattice, its material's **shaper** shapes the noise
into that material's pattern, and the result modulates the row the quad's
material slot already fetched. No geometry is added — that is the whole point
(C-122, C-91). The first form of this section addressed the subvoxel inside its
*block*, which stamps the same pattern into every block (the eye-check's
tiling); the world cell is the correction.

The 256 level is the **live** one: this read is static today, but the address,
the shaper and the rungs are exactly what a fluid will animate (its cells
interpolated between addresses) while a solid's non-addressed properties stay
this same static read — foam on the surface, caustics through the body, and a
shaded edge where a material leaves its block (the user's ruling, recorded in
`pattern.rs`).

The rungs are the model's **ladder** (C-163): the same field read in cells of
`2^octave`, each drawn only while the pixel's footprint is under half its cell,
summed with fBm gains and normalised — so a surface keeps structure at the scale
it can draw, and the flat patches of the retired placeholder tone are gone.

This section is a **transcription** of `voxel-appearance/src/detail.rs` and
`voxel-client/src/pattern.rs`, held to them by
`renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry`.
WGSL has no `u64`, so splitmix64 travels as (low, high) `u32` pairs; every
operation below is exact integer arithmetic, and the only difference from the
Rust is `f32` versus `f64` at the very end.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 435-463 (29 lines).*

### `pattern::octave_weight_staged`, route a (C-171)

`pattern::octave_weight_staged` (C-171 route a): the smoothstep is replaced by
its own single drawn point — a rung is **fully on** while the footprint is
strictly below the rung's Nyquist floor (cell/4, where the plain weight first
reaches 1) and **fully off** at or above it (the plain weight is already 0 at
the floor, so step and smoothstep agree at the boundary). A walk-sized step
moves the weight only across that one threshold, and the footprint is smooth,
so a rung flips as a coherent region crossing the eye's distance — the C-163
boil was a per-2×2-quad *derivative* artifact, and there is none of that left
here. Wider stages were measured to break the ladder: flooring raised weights
past the ceiling; ceilinging to quarter-cells or to powers of two blanked rungs
the plain weight still draws.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 634-644 (11 lines).*

### the deviation this fragment shows, with its gains applied

The deviation this fragment shows, with its material's gains applied and its
shaper shaping the noise — at the footprint the caller derived from the eye's own
geometry (`eye_footprint`): the whole of what C-161 draws, as C-162 corrected it
and C-163 smoothed.
Returns `(roughness, reflectance, coverage, 0)`. **`coverage` is the third
component on purpose**: the caller has to know whether any rung could be drawn
at all, because the answer \"no rung\" is not \"no detail\" — the per-vertex sample
answers there instead — and a caller that read only the first two components
could not tell the two apart. It is a `vec4` rather than a struct because WGSL
returns it by value either way and the padding is explicit.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 692-701 (10 lines).*

### the LODs: a reflectance that is a function of the Sun's angle

---- the LODs: a reflectance that is a function of the Sun's angle -----------------

**The user's ruling, 2026-10-10:** where a fragment cannot resolve an address, its material's
reflectance is to *correspond to the angle of viewing based on the sunlight* rather than be
removed. Two paths leave a fragment with no address at all — the **record** format carries no
corner sample (`sample_present` 0), and the ladder is deliberately silent past its ceiling
(C-163) rather than aliasing — and until this function existed both fell through to a hard
zero. That zero is flat albedo at distance: the look a before/after screenshot comparison
found missing, and the reason the report was worth more than any test that was passing.

Every input here is the fragment's own geometry — its normal, its view vector, and the beam the
rig carries — so the term is **continuous under any eye step**. There is no address and no
footprint, so there is nothing for a step of the eye to re-roll: that is exactly the property
the flicker repair needed, and the zero it got instead does not have it.

The shape is the boundary's own reflectance read at both of this fragment's angles: the share
the interface reflects **at the beam's angle**, `F(θ_L)`, unioned with the share seen **at the
view's angle**, `(1 − n·v)⁵` — the two angles the user named, and the same two the rim already
spends. It is **one-sided on purpose** — zero for a face meeting the light and the eye fairly,
growing as either goes grazing — because a term centred on zero would move the whole world's
tone as the Sun moved, and a material's colour is its own (C-165's rule).

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 850-870 (21 lines).*

### the material's own pattern, read at this fragment's address

The material's own pattern, read at this fragment's address: the block grid
and subvoxel grid a greedy-merged quad cannot carry as geometry (C-161).
**The footprint is the eye's own geometry, not a derivative.** `fwidth` is
estimated per 2×2 quad, so on a face the camera sees at an angle its estimate steps from
quad to quad; the ladder's weights stepped with it and the face boiled as the
camera moved (C-163). This is a function of the fragment alone.

The per-vertex detail is the lattice sampled once at the corner's own world
address (C-161), and the ladder below reads rungs from the fragment's own
footprint — so a merged quad shows sub-pixel detail while the vertex cost
stays constant. Which of the two the fragment shows is decided a few lines
down, by what the pixel can actually draw.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 922-933 (12 lines).*

### The ladder speaks where it can, and the vertex speaks where it cannot

**The ladder speaks where it can, and the vertex speaks where it cannot.**
The ladder is a weighted mean over rungs a pixel of this footprint can draw,
and it is deliberately *silent* past its ceiling rather than aliasing
(C-163): at that distance four shared corners are all the detail there is.
The per-vertex sample is that detail — one lattice value per corner, at the
corner's own world address, interpolated across the quad — which is the new
architecture's contribution to a surface the old one left bare. Adding the
two would count one address twice; `mix` at the ladder's own coverage keeps
the rung blend exactly as it was wherever the ladder draws, so a material
that shows its pattern today shows the same one.

Both sides carry the material's own gains (`ShadingField::gain`): the ladder
applies them to its normalised mean, and here to the vertex sample, so a
material the world does not vary reads as unvaried either way.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 935-948 (14 lines).*

### the parameters the CPU's model reads and the GPU had nowhere to put

---- the parameters the CPU's model reads and the GPU had nowhere to put -----

`openpbr::derive` computes every one of these and `npr::shade` reads every one,
and until this group existed they reached the row and stopped: the shader drew
the boundary with no reflectance of its own, no coat, and no medium. The
`_admitted` flags are the same "absent is not zero" rule every other slot
follows, and each value's *use* is transcribed from `npr::shade` below —
including `specular_f0`'s `unwrap_or(1.0)`, which is the model's statement and
not this file's.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 986-994 (9 lines).*

### which light: the world's field, or the rig

---- which light: the world's field, or the rig --------------------------

A corner says whether the shaper layer had a measurement for its cell
(`light_present`). Where it did, the surface is lit by **the light that
reached it** — the field's own beam and sky, per band — which is what makes
a covered cell dark and an open one bright without the shader inventing a
shadow. Where it did not, the rig's lights stand in, which is the Source 2
model this file transcribes and how every frame was lit before the field
existed. The two are one `if`: a frame with no field is bit-for-bit the
frame it was yesterday.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1028-1037 (10 lines).*

### the illumination terms, in their order (1 — diffuse)

1. Half-Lambert diffuse wrap on the beam, and the sky whole: the sky
   arrives from every direction, so there is no angle for it to wrap at.

   **The boundary's share comes out of the beam, at the beam's own angle**
   (`OpenPBR_Energy_Conservation` §§5–6): `F(θ_L)` is the share of this beam the
   interface reflects instead of transmitting, so it is what the substrate is drawn
   from and what the rim below spends — together they can never exceed the light
   that arrived. Reading that budget at the **view** angle instead would make a face
   seen at a grazing angle lose its diffuse *whatever the Sun was doing*, which is
   the "blocks are just lit by themselves" defect. The coat is a *layer* (§9), so
   its own reflectance comes out of the base too. The sky is left whole on purpose:
   it is a hemisphere, not one angle, so a single-angle Fresnel cannot state its
   share — the spec's own `E[f_specular]` is an integrated albedo, and says so.

   **The field carries radiance and no direction**, so the beam's direction comes
   from the rig — the same source the warp below takes its direction from, and the
   same world's Sun the shaper layer measured this radiance against. With no rig
   light there is no angle to measure, and the view stands in.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1047-1064 (18 lines).*

### the illumination terms (2 — rim)

2. Rim: Schlick's exponent, view-facing, scaled by the beam's own
   radiance at this cell **and by the block's own reflectance** — what reaches
   the eye at a boundary is that boundary's reflectance, so the rim carries the
   block's colour instead of overriding it. The `select` is `npr::shade`'s own
   rule for a block nobody has a spectrum for: it reflects the light, not an
   invented grey. **It spends this beam's budget rather than adding to it**: it is
   the reflected share (`beam_share`), shaped by the silhouette — so a backlit face
   is all rim and no diffuse, and substrate + rim can never exceed the light.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1094-1101 (8 lines).*

### the illumination terms (3 — the specular warp)

3. The specular warp: a step, and the threshold is the material's own
   roughness rather than a number this file picked.

   **The direction comes from the rig, the magnitude from the field.**
   The rig is this world's skylight — the same sun the shaper layer
   measured the beam from — so its direction is the honest one, and the
   field supplies how much of it arrived here. A radiance triple is not
   a direction: `normalize(direct)` would point at whichever band is
   brightest, which is a colour, not a place.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1115-1123 (9 lines).*

### the fluid reflection: the sky, seen off the boundary

---- the fluid reflection: the sky, seen off the boundary ------------------

**A fluid is not a matte sheet with a sheen on it.** Looking along water, most of
what reaches the eye *is* the sky reflected, which is why `voxel_appearance::fluid`
exists and why the sea read as a hole without it. `shading::fluid_surface_cpu` is the
Rust side of exactly this composition and `fluid::surface_radiance` states the law:
the body through the transmitted share, plus the sky's radiance from the mirror
direction through the reflected share. This is that expression, transcribed.

**What makes a material a fluid here is `transmission`**, its own OpenPBR parameter
rather than a list of material ids in a shader: a boundary that transmits is one whose
mirror image the eye sees, and a boundary that does not is not. The gate also keeps
every stated-material comparison intact, since those carry no transmission.

The mirror ray can point below the horizon — a steep face reflects the world, not the
sky — and this model has no world in a reflection yet, so that case adds nothing: the
reflected radiance is black and only the `(1 − F)` transmission of the body remains. The
same guard, in the same words, as `fluid_surface_cpu`'s.

**The gate is the model's own presence**, the same question the sky pass asks: a caller
that has written no model is owed the frame this pass drew before a sky existed, which is
no reflection. `sky_radiance_rgb` refuses a short model by itself, so what this asks is
whether there is a sky to reflect at all rather than whether some table covers a grid.

*— stood in `crates/voxel-client/src/shading.wgsl`, lines 1195-1217 (23 lines).*

### The astronomical body: one record, and a sphere that draws the phase

---- the astronomical body (C-184) ------------------------------------------

**A body is not a uniform.** The Sun and the Moon used to travel as fields of the sky pass's
own uniform — a centre, a half-extent and a radiance, three `vec4`s handed over every frame,
one of them re-uploaded whenever the camera moved. A body is **geometry in the world's own
stream**: a record of the same 32-byte stride a chunk's rectangles travel in, on an origin
of its own, drawn by the world's own pass. That is what lets the streamer place one, and it is
`voxel_client::quad`'s `BODY_KIND` and `BodyRecord` that carry it.

A body's record is a rectangle's own stride. `vs_body` plots it at the **far plane** and
`fs_body` intersects the **cuboid** the record names, giving every pixel the normal of the face its
ray enters — which is what draws a *phase* rather than one face's single incidence, and what makes
the drawn body the shape the photometry measured.

**Where the body stands and where it is plotted are two different numbers, and only the first
is the world's own.** The record carries the body at `voxel_shapers::scale`'s 1:8 ruling —
`sun::DrawnSun::body_record` converts the centre and the radius through `voxels`, so the Moon's
record stands 4.805e7 m away and the Sun's 1.87e10 m. No `[0,1]` `Depth32Float` buffer holds
that: at 1.87e10 m the depth is `1 − near/d = 5.3e-12`, which rounds to the plane's own `1.0` in
`f32` and is discarded by the pass's `Less` test. `vs_body` therefore plots the record's
**direction** at `camera::FAR_M` and scales the half-extent by the same ratio —
`half · drawn / distance` — which leaves the subtended angle exactly the world's own, at any
plot distance.

**There is one shape, and it is the cuboid.** `BodyCuboid` is the shape the photometry is
measured against — the solid angle the Sun's irradiance is divided by, and the body a shadow's
penumbra marches toward — and it is the shape the pass draws: `fs_body` is a slab test on the
record's cube, whose faces are the world's own axes, and the normal it shades with is the entry
face's (one of six exact vectors). The plot quad is `√3` of the cube's half-edge, because a rotated
cube's furthest silhouette point is a corner and the quad has to contain it; the body is the cube,
not the quad.

**This replaces a sphere that was inscribed in the cuboid, and the sphere was wrong in three
places at once.** It delivered `π/4` of the irradiance the law divided by the body's own
`4·(R/d)²` — a face-on square covers `4/π` times the disc inside it — so the drawn body and the
photometric law disagreed about the same body's solid angle; it put a curved terminator on a body
with six flat faces; and the `(1 − cos E)/2` lit fraction the Moon's aggregate test used to assert
is the law of a **sphere's disc**, not of a cube, whose lit area steps as the Sun crosses its faces.
The correction is the shape, not a brightness scale: nothing divides a face radiance by `4/π` here,
and no test band was widened to absorb the difference.

**And the law follows the drawn cube's orientation.** A cube of half-edge `R` seen along the unit
direction `u` covers `4R²(|uₓ| + |u_y| + |u_z|)/d²` — face-on the `4·(R/d)²` above, corner-on `√3`
of it — so `sun::cuboid_solid_angle_sr` is what `DrawnSun::of` divides the sky's irradiance by. A
law that took only the distance would make a cubed Sun brighten by up to 73 % as its corners turned
toward the eye, which is the same defect from the other side.

`renderer::tests::the_sun_is_drawn_as_the_cuboid_the_model_says` holds the drawn silhouette to the
record's cube per pixel and measures its coverage against the inscribed disc's (1.6× at the shipped
opening tick), and `renderer::tests::the_moons_phase_is_drawn_from_its_own_geometry` holds the drawn
lit area to the cube's own six-face closed form, with the sphere's disc law printed beside it as the
expectation this body does not have.

*— stood in `crates/voxel-client/src/shading.wgsl`, the astronomical-body block (C-184), and in
`crates/voxel-client/src/sun.rs`, `DrawnSun::body_record`/`DrawnMoon::body_record`.*

## from `crates/voxel-client/src/atmosphere.wgsl`

*moved from lines: 1-61, 170-173, 232-241*

### The air's transport, once, for every pass that draws light through it

**The air's transport, once, for every pass that draws light through it.**

#### Why this file exists

WGSL has no `include`, and a shader cannot call another shader — the same fact
`display.wgsl` is built on. It was never true of this build: the host has `include_str!`, and
`world.rs::pass_source` composes a pass with the files in front of it. So the sky's transport
lives here **once**, and both passes that need it are compiled with it:

* `sky.wgsl`, which draws the sky behind everything, and
* `shading.wgsl`, which reflects it off a fluid surface, and lights a surface by it.

Before this, the transport was **one file on the CPU** (`voxel_light::solver`) and **a table on
the GPU**: `Sky::directional_radiance` was evaluated over a 64×64 (elevation, azimuth) grid on
the CPU and the shader bilinearly interpolated it. That is one model with a *tabulation* of
itself, and it had two defects a table cannot avoid. A grid is sampled at 64 azimuths, so a
sunset's aureole is the grid's own staircase; and the sky the shader drew and the sky the light
rig was built from were two different evaluations of two different models — the dome was
`voxel_appearance::sky`'s Preetham fit while the rig's sun and moon came from the band table.

#### What is computed here, and what is not

The transport is `voxel_light::solver`'s, transcribed: the single-scatter integral in closed
form, the two media with their two geometries (the air is a scale height deep, the droplets
their own layer of tens of metres), the mixed phase, the sun's own transmittance with the
reference's twilight decay, and the night sky's measured floor. **Nothing here is a dial**: every
coefficient arrives in `sky_model`, which the CPU fills from the band table and the shapers' own
air — see `skylight::SkyModel`. A second copy of a band's extinction in this file would be a
second atmosphere.

What is *not* here is the shape of a body: the Sun's disc and the Moon's are drawn by `sky.wgsl`
from their own geometry, and this file knows nothing about them.

#### The row layout, and why it is rows

`sky_model` is a storage array of `vec4<f32>` rather than a struct, because a struct's padding is
the shader's own business and a buffer written by hand on the CPU has to agree with it. A row
layout is checked by [`sky_model_rows`]'s own count and by a test that holds this file's numbers
to the Rust that wrote them:

| rows | what they carry |
|---|---|
| 0 | `scale_height_m`, the droplets' extinction /m, the droplet layer's depth /m, the droplets' single-scatter albedo |
| 1 | the sun's own air path /m, its own droplet path /m, the twilight's share, the night sky's radiance W/m²/sr |
| 2 | the direction **toward the Sun**, `xyz`; the exposure over the world's own peak sun |
| 3 | Mie's asymmetry, the droplets' asymmetry, the grazing air mass, the band count |
| 4 | **the Moon**: its own air path /m, its own droplet path /m, the illuminated fraction of its disc, the Sun's own visible fraction (the eclipse) |
| 5 | the direction **toward the Moon**, `xyz`; unused |
| `6 + 3i` | band `i`: extinction /m, scattering /m, the sun's power in it W/m², its Rayleigh optical depth |
| `7 + 3i` | band `i`: its aerosol optical depth, the night floor's share, its red and green response weights |
| `8 + 3i` | band `i`: its blue response weight, the Moon's reflection factor in it |

The count is `6 + 3 · 12`: six header rows and three per band. `sky_model_rows` is that number
and a caller that uploads fewer rows is refused by [`sky_radiance_rgb`] rather than read past.

#### Why the Moon has rows of its own

Because **the Moon is a source in this transport and not only a beam on the ground**. Its light
is the Sun's, reflected off its own disc, so a night sky is moonlit and a crescent's glow is not
a full Moon's — and so that an eclipse, which takes the Sun's disc out of the sky, takes the
whole transport down with it (`row 4`'s `w`, which every band's solar power is scaled by).

*— stood in `crates/voxel-client/src/atmosphere.wgsl`, lines 1-61 (61 lines).*

### The Moon's own rows

**The Moon's own rows.** `moon_row.w` is the fraction of the Sun's disc that is *not*
covered — 1.0 on every ordinary frame, and 0.0 in a total eclipse — and it is read here
rather than folded into the band powers so that the sky, the beam and the light rig can
all be darkened by one number that came from the two bodies' own positions.

*— stood in `crates/voxel-client/src/atmosphere.wgsl`, lines 170-173 (4 lines).*

### The Moon's own light, and it is the Sun's light reflected

**The Moon's own light, and it is the Sun's light reflected.** Its power is the band's
own solar power (`a.z`) through the band's reflection factor (`c.y`: the Moon's declared
reflectance over π, through the disc's own solid angle) and the fraction of that disc the
Sun is lighting (`moon_row.z`) — so a crescent is dim and a full Moon is bright, and
nothing here is an emitter that the Sun does not feed. Its air path is **its own**
(`moon_row.xy`) and its phase angle is the one between the ray and `moon_dir_row.xyz`,
because the light reaching this pixel left the Moon and not the Sun. **No twilight
share**: that curve is the Sun's afterglow and the Moon is a body. And the Sun's own term
beside it is scaled by `sun_visible`, so an eclipse that takes the Sun's disc out of the
sky takes every band of this transport down with it.

*— stood in `crates/voxel-client/src/atmosphere.wgsl`, lines 232-241 (10 lines).*

## from `crates/voxel-client/src/sky.wgsl`

*moved from lines: 233-238, 252-255, 258-279, 288-299*

### A uniform with no model behind it draws black

**A uniform with no model behind it draws black**, which is the frame this pass drew
before a sky existed. The refusal is the transport's own: `sky_radiance_rgb` answers a
model shorter than its row layout with nothing at all, so a caller that never wrote one
is not an error case here with a second spelling.
The camera basis, matched to `Camera::basis`: yaw about +Y, pitch about the right
axis, no roll (the sky does not roll with the drawn camera).

*— stood in `crates/voxel-client/src/sky.wgsl`, lines 233-238 (6 lines).*

### What this ray carries

**What this ray carries**, from the one transport the sky and the world are both drawn
from: the sun's own bands through this air, at this pixel's own direction. The frame the
ray is in is the one `skylight::sun_direction` builds the model's sun in — `x` east, `y`
up, `z` south — which is the only reason the two can share an angle at all.

*— stood in `crates/voxel-client/src/sky.wgsl`, lines 252-255 (4 lines).*

### Transform first, then dither

**Transform first, then dither**, for the reason `shading.wgsl` gives: the
dither is about the steps that remain *after* the range is settled, so
dithering a value that is about to be rescaled dithers something that is not
on screen. Both are gated on the target, so a measurement frame is the
radiance the model computed.

The dither is applied to the colour and **not to the alpha**: alpha is this
pass's "geometry is here" channel and its value is a fact, not a shade — a
dithered alpha would read as a half-covered pixel that no geometry made.

**The Sun's body and the Moon's body are gone from this pass** (C-184). They are astronomical
bodies in the world's own stream — records in the scene's quad array, expanded by `vs_body` and
shaded by `fs_body`, both in `shading.wgsl` — so this pass carries the air and nothing else, and
its uniform has no body in it at all. The two paragraphs that stood here and the Moon's own
section that followed them described the retired uniform cuboid and the pass's own composition of
it; what replaced them is under *The astronomical body* in the `shading.wgsl` section above.

*— stood in `crates/voxel-client/src/sky.wgsl`, lines 258-279 (22 lines) and 288-299 (12 lines);
the bodies left the pass on 2026-10-11, and the lines that drew one are gone from the file.*

## from `crates/voxel-client/src/sun.rs`

*moved from lines: 317-321, 538-542, 981-984*

### Refused by name, not by how a comparison happens to treat a NaN

**Refused by name, not by how a comparison happens to treat a NaN.** `!(x > 0.0)` also
refuses a NaN, and that behaviour is wanted — a NaN radiance is not a sun — so the NaN
is refused explicitly here, which is the form `skylight::sky_lut` already uses for a
normaliser and the one clippy's `neg_cmp_op_on_partial_ord` asks for: `!(x > 0.0)` and
`x <= 0.0` differ for a NaN, and a reader cannot tell which was meant.

*— stood in `crates/voxel-client/src/sun.rs`, lines 317-321 (5 lines).*

### A tick the model itself calls a lit moon

**A tick the model itself calls a lit moon**: the body up and more than a third lit.
The tick used to be a constant, which the orbit invalidated — the Moon has a rise time
now, so a fixed tick is a tick with the body possibly below the horizon and no face to
draw (that is exactly what `DrawnMoon::of` refusing it means). The scan is over one
synodic month, so it is the world's own night rather than one picked for the picture.

*— stood in `crates/voxel-client/src/sun.rs`, lines 538-542 (5 lines).*

### And it is a division and not a multiply

**And it is a division and not a multiply**, said as the inverse: dividing the
exposure back out and multiplying the solid angle back in recovers the irradiance
share the radiance was made from. A wrong direction of that step fails here rather
than looking plausible.

*— stood in `crates/voxel-client/src/sun.rs`, lines 981-984 (4 lines).*

## from `crates/voxel-client/src/skylight.rs`

*moved from lines: 132-142, 150-164, 187-201, 208-246, 278-305, 559-576, 1650-1661, 1928-1937, 2089-2100*

### The sky is derived here rather than handed in

**The sky is derived here rather than handed in.** The signature used to be
`(spec, tick, sky, sun_altitude_deg)` — three free parameters for a function whose
own module doc says it "is a function of `(tick, atmosphere)` and has no free
parameters" — and the fourth merely restated `sky.sun_altitude_deg`, so a caller
could hand in a direction built from one atmosphere and a light built from
another. **Two sources of truth for one atmosphere was the defect the signature
had**, and it is precisely what made the normaliser below dishonest: the
denominator could not see the turbidity the numerator was measured through.

`None` rather than an unlit rig. A refused atmosphere is a question this model
cannot answer, and a black frame would be an answer nobody asked for.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 132-142 (11 lines).*

### intensity is normalised by this world's own peak sun

**Intensity is normalised by this world's own peak sun, measured through the
same atmosphere it is dividing**, so the rig carries a ratio rather than a number
in W/m². The shader multiplies a half-Lambert term by this, and a material's
albedo, and an 8-bit swapchain has no use for 1200 W/m².

**What was wrong here, and why it read as a small number rather than a wrong
one.** The divisor used to be `noon_irradiance_w_m2(spec)` — the *geometric*
direct irradiance at the year's highest noon, which never sees the air path or
the turbidity — while the dividend was `sky.sun.luminance()`, which is always the
sun **after** `exp(−β_R·H·m) · exp(−β_M·T·H·m)`. Both are W/m², so the units
matched and nothing looked wrong; the two sides were simply two different
measurements, and the missing air column was 9.93% of every intensity this rig
has ever produced. That is the sense in which the normaliser normalised nothing:
it was normalising this world's sun by a sun that had not been through this
world's air.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 150-164 (15 lines).*

### the colour is a chromaticity, not a radiance

**The colour is a chromaticity, not a radiance — and that is the whole of the
103×.**

`Light` is `colour` × `intensity`: `npr.rs` multiplies them and `shading.wgsl`
computes `base * (light.colour.rgb * lambert)` where `lambert` already carries
`light.direction.w` — the intensity. So one of the two carries the magnitude
and the other carries only the shape. [`LightRig::standard`] settles which is
which: colour `(0.85, 0.90, 1.0)`, intensity `1.0` — a chromaticity whose
strongest channel is exactly 1, and a magnitude beside it.

**The sky's sun was handing this shader `sky.sun` itself**, which is the
transmitted radiance in W/m² and measured about **1090**. So the light was a
1090-magnitude radiance multiplied by a 0.78 ratio, and the world rendered
`mean 0.87457` with a brightest pixel of `78.3`. The fix is not a smaller
number; it is putting each factor in the place the rig already put it.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 187-201 (15 lines).*

### The moon is a night light, and only a night light

**The moon is a night light, and only a night light.** It is added when the
Sun is **down**, which is the whole of `MAT-0012`'s claim: a night lit by a
second source rather than by the sun multiplied by a small number.

The Moon is also above the horizon at noon — that is a fact about the sky,
not about this rig — and a first version added it then too, so noon carried
two lights. That is wrong for a reason worth stating: the Moon's light at
noon is ~1e-5 of the Sun's, so the second light costs a shader loop
iteration to contribute nothing visible, and worse, it is a *lie about the
physics of a shadow*: a shadow cast at noon would be lit on one side by the
Sun and, invisibly, by the Moon. The rule is therefore the Sun's own state,
not the Moon's.

**And the Moon is the Sun reflected, so this light is the solver's own answer rather than a
second model of the body.** Per band it is the solar constant's power in that band times the
band table's **declared reflectance**, over π by Lambert's law, through the disc's own solid
angle and back down the same air the Sun crossed ([`voxel_light::moonlight`]) — so doubling
the Sun doubles the moonlight and an independent emitter could not. **The drawn body already
came from the same bands** (`sun::DrawnMoon::of` reads `moon_radiance`), so what this
replaces is the shapers' `sky.moon`: one body described twice, once for the picture and once
for the light it casts, with nothing comparing the two.

Its **direction is toward the Moon** and not toward the Sun — a moonlight shadow points away
from the body casting it, and a light aimed at a body that is not in the frame is a light
whose shadows are wrong in the one frame they exist in. The direction is the **orbit's own**
(`voxel_shapers::sky::moon_position`), the same reading the drawn body is placed from, so the
light and the body cannot be in two places.

**And a Moon below the horizon casts nothing, which the old zenith could not say.** The
guard is not a second rule: `moonlight` returns all-zero bands for a body the solver refuses
(`moon_radiance` has no radiance below the horizon), so the `total > 0.0` below is what a set
Moon fails. A waning crescent that rises an hour before dawn therefore leaves the hours
before it genuinely dark, which is the physics of a body with a rise time.

**The magnitude is the real one and it is small**: a full Moon delivers about 3e-3 W/m²
against the Sun's ~1.3e3, so the night's light is five or six orders under the day's. That
is the model's own ratio rather than a dial, and what makes a night *readable* on a display
is the eye's adaptation rather than a brighter Moon — a separate piece, named in
`docs/light-solver.md` and not smuggled in here.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 208-246 (39 lines).*

### Divided by the same peak the sun's intensity is divided by

**Divided by the same peak the sun's intensity is divided by**, because a rig
carrying a ratio in one field and a raw radiance in the other is one rig in two
units. The user saw it on screen: the world rendered **103× too bright** —
`mean 0.87457` against the fixture's `0.00847`, brightest pixel `78.3` — and it
lost colours, 6 against the fixture's 8, because an ambient four and a half
times brighter than full scale swamps the material differences the shading
exists to show.

**So the ambient is now a ratio too**, and it moves the way the sun's does:
brighter near sunrise (where the path is long and the dome genuinely scatters
more) and dimmer at noon. It measured `17.97` at the client's start tick
against `4.62` at noon, which is the physics; the divisor is what makes those
two numbers commensurable with an intensity of `0.52` and `0.78`.
**The ambient is the atmospheric model's own radiance, and not the dome's.**

The two ends are still the horizon and the zenith, and the share is still
[`AMBIENT_FROM_HORIZON`] — but the *reading* is the transport `atmosphere.wgsl` draws,
which is where the sun's afterglow, the Moon's reflected light and the night's measured
floor all live. The dome it replaces had one source and therefore one behaviour: zero
below the horizon, always, because the Sun alone was in it. So the world's ambient left
at sunset while the sky above it did not, and a midnight with the Moon 59° high and
nearly full measured an ambient of exactly **`0.0`** — the frame went out at moonrise
rather than at dusk, which is what the black night was.

**The dome is consequently deprecated for the atmosphere**, and this is the last reader
it had on the lighting path. `sky.wgsl` already draws the model; `atmosphere.wgsl`
carries it for the fluid's reflection too; and `voxel_appearance::sky`'s Preetham fit
keeps only its own tests and the drawn Sun's colour.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 278-305 (28 lines).*

### How far past a chunk's own face a hop steps before the next one is named

**How far past a chunk's own face a hop steps before the next one is named.**

A point *exactly* on a chunk boundary belongs to the chunk behind the ray — the
coordinate is a floor of the exact face — so a march that resumed there re-entered the
chunk it had just left, and the box test of a ray starting on its own box face answers
`None`. The first version read that `None` as "nothing more in the way" and left the
transmittance where it stood, so a wall standing exactly on a chunk boundary leaked: a
cell walled in by twenty-six solid voxels at a generated world's chunk edge came back
with `3e-4` of the sky in its diffuse bands, all three of them, along the one stencil ray
whose direction pointed out of the chunk. `voxel_client`'s `measured-light` example is
what found it, and `voxel_stream::light`'s
`a_cell_walled_in_by_rock_reads_no_light_from_any_direction` is the same property inside
one tree, where it held all along — the leak was the hop and not the walk.

A **micron**: four orders below a voxel's own 15.6 mm subvoxel, so no address a voxel or
a subvoxel question is asked about can move, and six below the shortest path length a
chunk can contribute. What it costs is that much path charged twice, which is darker by
an amount no measurement in this project can resolve.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 559-576 (18 lines).*

### What the user saw on screen

**What the user saw on screen.** The sun's intensity was a ratio against this
world's own peak — `1.0` at its brightest — and the ambient was the sky's raw
dome radiance, which measured **4.61765** at noon and **17.97** at the
client's start tick. So the rig carried a ratio in one field and a radiance
four to eighteen times brighter than full scale in the other, and the world
rendered **103× too bright**: `mean 0.87457` against the fixture's `0.00847`,
brightest pixel `78.3`, and *fewer* distinct colours than the fixture (6
against 8) because an ambient that dominant swamps the shading.

**Every value in a rig is now in `[0, 1]`.** That is the whole claim, and it is
checked across a whole day rather than at one tick, because an ambient that is
fine at noon and enormous at dawn is still wrong.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 1650-1661 (12 lines).*

### An assertion that was here, and why it is not

**An assertion that was here, and why it is not.** There was a
`RAINFALL_REFERENCE_MM > 0.0` check here, and clippy is right that it could never
fail for any value either side would plausibly take — a comparison between two
constants is not a pin. The pin beside it, `RAINFALL_REFERENCE_MM ==
weather::BASE_PRECIPITATION_MM`, is the one that catches drift, and a zero in the
shaper is that shaper's own test to catch. **I wrote an assertion to defend an
assertion, and deleted the defence when I read it again.**

**Both are gone as of the column slice**: turbidity reads a cloud's water path, so
weather's rainfall baseline is no longer a number this file depends on at all.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 1928-1937 (10 lines).*

### The absence is the point, and it is the Sun's absence that is the point

**The absence is the point, and it is the Sun's absence that is the point.** A rig that
kept the sun's light through the night would light a night as though it were an overcast
afternoon, and the difference between those two pictures is the entire reason the sky
exists.

**What this test used to assert was the defect.** It read *"and a black ambient"* and
required `ambient == 0.0`, which is what the deprecated dome did — one source, so zero
below the horizon, always. Measured at a midnight whose Moon stood 59° high and nearly
full, that assertion held, and the frame went out with it: the world had no ambient at
all until sunrise. The ambient now comes from the atmospheric model, so the night is the
night floor rather than nothing, and it **rises with the Moon** because the Moon's light
is the Sun's reflected through the same air.

*— stood in `crates/voxel-client/src/skylight.rs`, lines 2089-2100 (12 lines).*

## The Sun's own light is not the ground's share

`voxel_appearance::sky::Sky::sun_light` returns **the Sun's own output**, and *whether this ground
can see it* is a different question answered elsewhere. These used to be one question: the function
returned `BLACK` the moment the Sun crossed the horizon, which made the Sun a lamp with a switch
where the world has one with a horizon. Two things were wrong with it, and they are the same
mistake — **the Moon is lit by this light** (its illuminance is the solar constant reflected by the
Moon's own surface, so a Sun that goes out takes the lit Moon with it), and the *frame* stepped,
because a source that returns `0.0` in one update is a step no exposure can follow.

So the air column is read at the Sun's own altitude, **frozen at the horizon below it**: a depressed
Sun's light does not cross a longer path, it crosses the horizon's own path in a sky whose lit air is
shrinking. That is the same rule `crates/voxel-client/src/atmosphere.wgsl` and `SkyModel::of_graded`
already carry, so one atmosphere has one answer in all three places rather than three — `sun_light`
previously said `0.0` where the shader said `air_mass(0)`.

The **ground's** share of the beam is `voxel_client::skylight::daylight_of`'s decision, and it makes
it with the twilight fraction: a surface sees the Sun's light decay through civil, nautical and
astronomical twilight rather than stop. The claim is pinned by
`voxel_appearance::sky::tests::the_suns_own_light_is_not_a_switch_at_the_horizon`, and the dome is
still the named absence it was — `dome_radiance` is a separate function and stays black below the
horizon, which is a claim about the *dome* rather than about the Sun.

## The horizon's streaming rules

The streamed light horizon (`crates/voxel-client/src/horizon.rs`) is a window on world space, not a
cache that is rebuilt: a 96 m cube of subvoxel occupancy in the fine band, an 8 m summary over 256 m
behind it, anchored on the eye's own chunk minus one so the eye stands in the middle.

**A chunk crossing shifts the cube; it does not clear it.** Two thirds of the world in the cube is the
same world one chunk over, so `recenter` copies both bands to their new addresses — one row copy per
line, the far band's 32 m step falling on four whole 8 m cells — and keeps every stamp whose chunk is
still inside the box. That is what makes the per-frame stamp budget cheap: a crossing owes only the
chunks that entered, which is why `HORIZON_CHUNKS_PER_FRAME` is nine and not twenty-seven. The version
that cleared the cube and restamped all twenty-seven walked every leaf of three unchanged chunks in the
frame the player crossed a boundary, which is a stall the eye reads as the game hitching at a chunk.

**A slot that has not been stamped is unmeasured, never air.** The coverage mask carries one bit per
chunk slot; where it is clear the march reports no measurement and the shadow map keeps the answer. So a
partially filled cube is a box with some of its answer delegated, and never a chunk-shaped hole in the
light.

**The bit means walked at 1 m, not merely recorded.** A chunk stamped while it was outside the cube has
a record whose only band is the coarse one, and counting that record set the slot's bit with every cell
gone: the frame then read zeros as *measured air* and the chunk beside the player reported a full view
of the sky from every metre in it. That is the one shape of this defect the mask itself carried — a bit
is now set only by a stamp that wrote the fine band, so a chunk waiting its turn is unmeasured and the
shadow map answers instead of an invented dome.

**A chunk that leaves the window is forgotten from both bands — its own box only.** A coarse cell is
8 m and a chunk is four of them aligned to its corner, so no neighbour shares one. Clearing the whole
coarse band instead (which the first version did) threw away every other chunk's summary while leaving
their versions current, so the far field went blank on the frame the window moved and stayed blank.

**Stamps are keyed by the version of the tree they were taken from and the bands they wrote**
(`voxel_work::Streamer::tree_version` plus the record's own bits), so an edit's rebuilt chunk is stamped
again on the frame its new tree lands, and an untouched chunk is never walked twice. The version alone
was not enough: a chunk is first seen eight chunks out, where a stamp writes only its 8 m summary, and
the same chunk is the one the player walks into later at the *same* version — with no 1 m cells at all.
`wants` therefore asks two questions, "is this the tree I stamped?" and "did that walk reach the rung
this chunk is at now?".

**A crossing drops from the record every band it cut out of the volume.** Leaving the cube shifts a
chunk's metres out of it, so the fine cells are gone; the record loses that band with them and the chunk
is walked again on the way back in. What survives a crossing is the coarse summary, which is what keeps
a re-entry to the chunk's own walk rather than a rebuild of a band.

**The coarse band is a level of the same octree, not a mask.** Each 8 m cell carries the share of its
512 voxels that hold matter (bits 0–7) beside the stamped bit, accumulated during the same leaf walk
the fine band is stamped from: a `Uniform` leaf adds all of its volume, a `Mixed` leaf its solid
subvoxels, and a leaf wider than eight metres divides its volume across the cells it spans. The
mesher itself meshes every leaf at its own level (`mesh::leaf_region`), so the coarse band reads the
same leaves at the level of its own cell — no second ladder is invented, and the share is exactly the
mean of the fine band over that cell, which is why the two agree where they meet instead of
disagreeing at the ring.

The march composes the bands by transmittance, `(1 − fine) · Π(1 − share)`: eight metres that are a
third full dim by a third instead of blocking the ray, and a cell that is a one-voxel roof is 3 % of a
wall rather than a wall. That is the rule the material ladder already states (`pattern::octave_weight`
gives a rung the share it covers), applied to occupancy instead of to a material's detail. The direct
beam still ignores the coarse band entirely; only the dome reads it.

## The far surface: a chunk at the coarse cell's own level

The light horizon answers *how much sky a cell sees*; it does not draw anything. The drawn far
field is a second, separate subject, and it has its own owner: `crates/voxel-client/src/far_surface.rs`
(`FarSurface`), with the horizon's shader extracted to `horizon.wgsl` so that neither file carries the
other's subject.

**One chunk, two levels, and one rule.** `coarse_mesh` walks the chunk's own octree at
`FAR_LEVEL` (3 — an 8 m cell, `2^3` voxels) with the octree's `visit_at_level`, which reports the
same regions whether or not the tree compressed there. A cell is drawn when **at least half of it is
matter** (`COARSE_MAJORITY`) and takes the material holding the largest share of it, tie broken by
the lower id. That is the octree's own "a region is one material" rule read at level 3 instead of
level 0, so there is no second classifier: a cell that is 3 % occupied is *not* drawn at this level
and the fine band still draws it, which is the one place the levels legitimately disagree and it is
stated rather than hidden.

**Volumetric, not a heightfield.** The cells are octree regions in three axes, above and below the
surface, so rock, water and air are classified the same way the fine band classifies them. Nothing
in this module addresses a chunk by `(x, z)`; the terrain is a volume and the far level samples that
volume coarsely.

**The mesh is the chunk's own frame.** Quads are emitted in subvoxel coordinates (a cell face is 32
subvoxels across), at the chunk's own origin, with the same winding and material as the fine
mesher's. No new vertex format, no new origin field and no new pass: the far meshes travel into
`ResidentScene::sync` beside the fine ones. A face on the chunk's outer boundary *is* emitted — the
neighbour's tree is not held here, and a hole in a distant cliff is worse than two coincident quads
buried inside the solid.

**The handover.** `FAR_HANDOVER_CHUNKS` is 3: a chunk inside that Chebyshev radius is drawn fine,
outside it the coarse mesh replaces it. One chunk is never drawn at both levels, so there is no
double draw to z-fight and no ring the coarse mesh has to clip against; both meshes share the same
chunk box, so the seam is a boundary rather than a gap. The jump is a quantisation of the same
surface, not a different one, and how visible it is at 96 m is a measurement rather than an
argument.

**Streaming, and what an absent slot means.** `FarSurface` keeps one entry per resident chunk,
keyed with the version of the tree it was built from — the same `Streamer::tree_version` the stamps
use. A chunk that leaves the window loses its entry and keeps nothing; a chunk that is resident
without an entry is **absent from the frame**, not drawn as a stand-in. `main.rs` builds at most
three a frame, so a crossing is the chunks that entered rather than a rebuild of the far field.

**Not proven without an eye on the screen:** whether the level-3 surface reads as terrain rather
than as stair-steps at the ring; whether the 96 m handover pops in motion; and what the far field
costs as a share of a frame. The tests prove the geometry, the share rule, the residency and the
level choice — not the picture.

## A chunk is drawn only with sampled light

**The rule.** A chunk whose horizon cells have not been stamped is **not drawn**: it is absent from
the frame, exactly as an unmeshed chunk is, and it is retried on the next frame. It is never drawn
on the rig's flat ambient as a stand-in.

**Why the rule had to be written.** A chunk that streamed in was handed to the scene before its
light had arrived, so its fragments fell through to `rig.ambient` — one number for the whole frame —
and the chunk read as unlit against the chunks beside it. That is the whole visible defect: not a
wrong light, but a *missing* one with a plausible-looking substitute behind it. The same rule the
far surface already obeys applied here: unfilled slots are unmeasured, not air.

**Where it lives.** The frame's stamp pass runs before the residency sync
(`crates/shploingus/src/main.rs`, the horizon block), and the chunks still wanting a stamp inside
the horizon cube are filtered out of `drawn_meshes()` before `ResidentScene::sync` sees them. The
cost is one map lookup per resident chunk per frame and no new state; the budget that stamps nine
chunks a frame is unchanged, so a chunk that lands during a crossing becomes drawable within one or
two frames.

**What this trades.** The demo walk's "the chunk under the eye is drawn on the frame it arrives"
count now measures *lighted* arrivals: a chunk the streamer holds and the light has not stamped is
counted absent, because it is. That is the honest reading — a chunk drawn without its light was the
defect, not the arrival.

**What it does not claim.** Cells outside the horizon cube (96 m around the eye, three chunks a
side) have no stamp and no rule; a fragment there takes the frame's sampled sky and the shadow map,
which is the far field's own answer rather than a chunk's. The cube is the boundary of the claim.

## The dome reads the fine band only

**Corrected 2026-10-25.** The dome's five stencil rays used to run to `HORIZON_FAR_REACH_M` and
continue into the coarse band, so a cell's *ambient* was a view fraction quantised to eight
metres. On a surface whose own detail is a thirty-second of a block that reads as a false edge
beside every cliff and every step in the terrain — a patch of light or shade eight metres across,
in the wrong place, wherever the coarse grid disagreed with the geometry. The coarse band is a
**beam** instrument: it lets a long ray be stopped at a kilometre, and it is eight times cheaper
per metre *because* it is eight times coarser, which is exactly why it cannot answer a dome.

The dome now asks `HORIZON_REACH_M` and the coarse band is never walked on its behalf, so the far
path in `horizon_transmittance` is entered by no caller. A corridor longer than the fine box reads
open — the answer this had before the coarse band existed, and the one a quantised dome was
hiding. What is lost is stated rather than papered over: ambient occlusion past 96 m is not
measured, and the fragment falls back to the frame's sampled sky there.
