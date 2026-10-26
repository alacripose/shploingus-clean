# PATCH-050 — the cadence that has already happened

The mission this pass was given, 2026-10-22:

> *"The shipped binary still rebuilds the entire scene on the atmosphere cadence — roughly five
> times a second at ~23 ms each — and that is the only remaining periodic stall in an otherwise
> 10–11 ms frame … Trace the invalidation path (SceneKey/light/atmosphere tick → rebuild → full
> corner re-upload) … and make a frame that only carries changed light stop paying a full geometry
> rebuild … Done means: the shipped binary, run idle and under eight CPU hogs, shows no frame whose
> build cost is attributable to the atmosphere cadence …, identical output pixels/digest against the
> current tree, memory no higher than now, and the new behaviour pinned by a test that fails if the
> full rebuild returns."*

# What the trace found, and what the premise got wrong

The path is real and it is one line long:

```rust
light: self.rig_at_tick.unwrap_or(0),      // SceneKey, built every frame
```

`SceneCache::take` rebuilds when the key moves, and `rig_at_tick` moves every four ticks — so every
cadence frame walked every resident mesh and re-sent the corner buffer. **But the rebuild was not
23 ms, and it was not where the judder came from.** `SHPLOINGUS_FRAMES=1` on the shipped binary,
idle, splits the cadence frame like this:

| stage, on a cadence frame | ms |
|---|---|
| `sky-light` — the field's solver + `daylight_of` + the rig write | **14.3–14.7** |
| `sky-dome` — the sky model + the two body records | **4.2–4.3** |
| `scene` — the scene cache's rebuild | 1.4–3.6 |
| **total** | **34–44** on a frame that otherwise draws in 10–11 |

Splitting inside the cadence with a throwaway instrument (`CADENCE_SPLIT`):

| inside the cadence | ms |
|---|---|
| `CellField::of` → `WorldLight::of` (the beam, the five dome rays, the table) | **11.5–12.2** |
| `daylight_of` — the same atmosphere, **derived a second time** by the frame | 2.2–2.3 |
| sky model + bodies | 2.8–4.3 |

So the ~19 ms the player feels is **the atmosphere's own derivation**, and the scene rebuild is a
small tax on top of it. Two more facts fell out of the trace and they changed the shape of the fix:

- **The field the rebuilds exist for measures nothing.** The binary's own report, on every run:
  `light-field 0 cell(s) marched | 0 corner(s) answered … | 1 512 244 corner(s) rigged`. So the
  scene the cadence rebuilt was **byte-identical to the one already in hand** — the rebuild was pure
  cost, and the light key's comment ("the tick the rig and the light field were derived at, so a
  frame built in one atmosphere cadence is not kept into the next") was describing a change no
  corner could carry.
- **`WorldLight::of` derives the daylight again internally**, so one cadence derived the same
  atmosphere twice and discarded the first copy.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/atmosphere.rs` (**new**) | `Atmosphere` — one tick's derivation as one value (inputs, `Daylight`, `WorldLight`, `SkyModel` + peak, the two `BodyRecord`s) — and `AtmosphereAhead`: a worker thread, one ask per cadence, and `cadence()` which **adopts the answer only when it is the same derivation in every argument** and otherwise derives here, counted. `sky_model_at` moved here from the bin, so the dome and the bodies are derived where the rig is |
| `crates/voxel-client/src/skylight.rs` | `WorldLight::with_daylight` — the solver from a `Daylight` the caller already has, so the cadence derives one atmosphere instead of two |
| `crates/voxel-client/src/field.rs` | `CellField::of_light` — the field around a solver the caller already has, carrying the previous cells by the same rule |
| `crates/voxel-client/src/scene.rs` | `SceneKey::light_of(held_cells, rig_tick)`: **the cadence moves the light key only when the field holds a measurement.** A field that holds nothing leaves every corner on the rig, and the rig is a uniform written every cadence whether the geometry moved or not |
| `crates/shploingus/src/main.rs` | the cadence block is now *sample the inputs, `cadence(…)`, write what came back*; the ask for the **next** due tick is made every frame (idempotent) with the inputs that cadence will be derived in; `ATMOSPHERE_LEDGER=1` prints one line per cadence (adopted/derived, cost, how many the frame had to derive) |
| `crates/voxel-client/src/scene.rs` (tests) | two pins: four cadences and **one build**; and a field that measures nothing gives **byte-identical corners** at two ticks |
| `conformance/missing-docs.txt` | `voxel-client`'s and `shploingus`'s pins re-taken, both counts unmoved (191, 0) |
| `docs/STATUS.md` | a measured row for the cadence |

# The design in one paragraph

The next cadence is known four ticks before it is due, and the derivation is a pure function of
`(spec, tick, inputs, eye)`. So the frame asks for it as soon as it can, the worker derives it while
four frames draw, and the frame that is due **adopts a value already in hand** — 0.1 ms instead of
20. Nothing is approximated and nothing is blended: `AtmosphereAhead::cadence` takes an answer only
when the tick, the inputs **and** the eye are the ones it is about to use, so an adopted atmosphere
is bit-for-bit the atmosphere the frame would have derived. For the geometry, the rule is the same
idea from the other side: the key is a function of what a corner can carry, and a field that holds
nothing cannot carry anything.

**Where the inputs come from is the subtle half.** The ozone is a *held* daily quantity, so a worker
that sampled the world itself would be answering a different atmosphere than the frame asked about
(the first version did exactly that: every cadence mismatched on ozone's fourth decimal and fell
back). The ask therefore carries the inputs: the turbidity sampled *for* the target tick at the
player's column — it is a pure function of the tick, so this is exact — and the held ozone.

# The numbers

Same machine, dev profile, `ATMOSPHERE_LEDGER=1 SHPLOINGUS_FRAMES=1`, 70 s runs, the settled window
(605/605 chunks, `in flight 0`), and the same tree before and after this pass.

| | before | after |
|---|---|---|
| cadence frame, `sky-light` | **14.3–14.7 ms** | **0.00 ms** (adopted) |
| cadence frame, `sky-dome` | **4.2–4.3 ms** | **0.00 ms** |
| cadence frame, `scene` | 1.4–3.6 ms | **0.00 ms** |
| cadence frame total | **34–44 ms** | 8–11 ms (the same as any other frame) |
| the cadence's own cost, adopted | — | **0.096 ms mean, 3.24 ms max** idle; 0.083 / 1.20 loaded |
| cadences adopted | — | **273 of 274** idle, **196 of 197** loaded |
| the one that is not | — | **boot**, tick 9000, before any answer can exist: 11.9 / 14.3 ms, `sync 1` |
| frame, idle | 10.8–11.7 ms, **91 fps** | **7.9–8.2 ms, 123–124 fps** |
| frame, 8 hogs | — | **8.2 ms, 120 fps** |
| stalls over 25 ms, settled window | **5–6 per 90 frames** | **0 per 125 frames** idle; 0 in the loaded window's last interval |
| worst frame of the run | 109.8 ms (a boot frame) | 216.5 ms at t9000 — **the boot frame**, shell 149.9 + draw 19.1 |
| worst frame after the world settles | 109.8 ms | **no stall at all**: every frame in the settled window is under 25 ms |
| world digest | `7c66a632336cc0a3` | **`7c66a632336cc0a3`** |
| scratch | 0.0 MB | 0.0 MB |

The last two rows are the identity claim: the same digest at 605/605 chunks, before and after, and
no scratch growth. The per-frame ledger the mission asks for is `SHPLOINGUS_FRAMES=1`'s `stall` and
`frame-summary` lines; in the settled window `sky-light` and `scene` are **0.00 on every frame**,
including every cadence frame.

# Identity, argued from the code and pinned by tests

- **The derivation.** `AtmosphereAhead` runs `Atmosphere::of` — the same function the frame runs —
  and `take` returns it only when `tick`, `inputs` and `eye` match to the bit. The test
  `the_look_ahead_is_the_same_derivation_and_the_frame_adopts_it` compares a **fingerprint over
  everything a frame can see** (inputs, grade, the field's beam, the dome's own bytes, the bodies'
  centres and radiance) between the worker's value and the frame's, and asserts the frame **derived
  nothing** when it adopted. A revert to deriving on the frame makes `derived_here` increment on
  every cadence and fails it.
- **The geometry.** `a_field_that_holds_nothing_does_not_rebuild_on_the_atmosphere_cadence` takes
  four cadence ticks through one cache and asserts **one build**; `an_empty_field_gives_byte_identical_corners_at_every_cadence_tick`
  builds the same face at two ticks through a real `CellField` over a provider with no chunks and
  asserts the **corner and index bytes are equal**. That is why skipping the rebuild changes no
  pixel: the bytes it would have produced are the bytes in hand.
- **The binary.** The world digest is unchanged (`7c66a632…`) and the run reports `0
  stale-but-drawn`.

# What it does not do, named

- **A cadence whose eye moved since the ask is derived on the frame.** The look-ahead is asked with
  the eye of the frame that asks; a player who walks between the ask and the due tick gets a
  mismatch, and the cadence falls back to the old path (counted, `sync`). Idle and standing still —
  the measured cases — never miss. Making the walking case free needs the derivation to be split by
  what each part depends on, not a bigger look-ahead.
- **Boot still derives once, synchronously** (11.9–14.3 ms on a frame that is already 150–216 ms
  because the shell prepares its first frame). There is no earlier frame to have asked from.
- **The field still measures zero**, and that is a real defect this patch deliberately did not
  touch: `Scene` asks `CellField::at` at **subvoxel** cells (`world_cell` = `floor(p / 0.25 m)`),
  while `at` treats the cell as **metres** (`centre = cell + 0.5`) and `key_of` divides by
  `CHUNK_SIDE` as if it were voxels. Fixing the address law would make the field answer, which would
  change every measured corner — the opposite of this pass's identity requirement. It is the next
  pass's work, and it is now cheap to reach: the rebuild it needs is the one this pass stopped
  paying for nothing.
- **`sky-dome` and the bodies are still built on the frame** in the fallback path; in the adopted
  path they arrive with the value.

# Verified

- `cargo test -p voxel-client --lib`: **515 passed, 0 failed, 2 ignored** on the final tree. One
  earlier run of the same binary failed `scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else`,
  the wall-clock-ratio test PATCH-049 already reported as intermittent on this machine (98 % disk,
  12 GB free); it passes alone in 0.02 s and passed in the final full run. It was not touched.
- `cargo test -p voxel-client --lib atmosphere::`: 1 passed. `… --lib scene::tests::a_field` and
  `… ::an_empty_field`: 1 each.
- `cargo build -p shploingus`: exit 0, so the build gate's census, conformance, prose frame and
  missing-docs pins hold on the final tree (both pins were re-taken with the recipe in
  `conformance/missing-docs.txt`'s header).
- The shipped binary, 70 s idle and 70 s under eight busy cores: the numbers above; `605/605
  chunks`, `0 stale-but-drawn`, `scratch 0.0 MB`, exit by `taskkill` only.
