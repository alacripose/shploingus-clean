# shploingus-clean

A fresh implementation of **Shploingus** — a voxel world with real terrain physics,
developed from research papers rather than ported from another tree.

This repository is **clean-room in the sense that matters**: every line here was
written here. Reference projects listed in `references.md` (Create, CubicChunks,
C2ME, Distant Horizons, Voxy, Sodium, Jade, Tinkers, and eighty more) are read as
*architecture* and *behaviour*; their files are not copied, and their licence
verdicts are per-project (`docs/MODS-MAP.md`).

## What is here

| Crate | What it owns |
|---|---|
| `voxel-model` | signed coordinates, the 4×4×4 subvoxel model, 256× material detail, identity, refusal |
| `voxel-noise` | the noise primitives every shaper is built from, each from its paper |
| `voxel-shapers` | the terrain shapers: tectonics, geology, terrain, erosion, hydrology, climate, weather, caves, substrate, ecology, delta, transition |
| `voxel-mesh` | sparse voxel octree + greedy visible mesh |
| `voxel-stream` | a `WorldSource` the mesher can read, the four horizons, the resident set |
| `voxel-instrument` | the one sensor concept, the min/max band, the weighing gate, the gauge wall |
| `voxel-appearance` | OpenPBR inputs derived from block truth, the four Source 2 terms over them, the 256× detail lattice addressed |
| `voxel-work` | the worker pool, the box-shaped streaming window, and the memory budget — the only crate that spawns a thread, and it depends on nothing outside the workspace |
| `voxel-client` | the GPU: meshes and a camera in, pixels out, **offscreen** — with the shader held to `voxel-appearance` by a test that compares the two on the same surface, and the HUD (`glyphon` text plus a rounded-rect panel pipeline) so a frame is drawable *and* measurable without a window |
| `voxel-ui` | the design system — Material Design 1 with the Human Interface Guidelines overlay, every constant labelled *MD1 rule* / *MD1 example* / *platform translation* / *project extension*, and **zero dependencies**, because a design system is a set of statements about numbers and statements about numbers can be checked without a display |
| `voxel-package` | content intake: a package is admitted by validation, never by optimism. Integrity and identity are separate types because a checksum is not a signature |
| `shploingus` | the window — the only crate with a windowing dependency, and the only place a person sees the world |

## The two rules that govern everything

1. **Refuse, never default.** A value that cannot be validated is refused and
   named, never replaced by a plausible one. A plausible substitute is
   indistinguishable from a correct one until much later — and by then it has
   been written into a world.
2. **Determinism is a contract.** The world is a pure function of
   `(seed, tick, declared period)`. Two runs over the same inputs are
   byte-identical, and that is a test, not an aspiration.

## Continuing development

Start with `docs/HANDOFF.md`: current implementation, verified commands, the next concrete task, and remaining alpha gates. It is deliberately short enough for a weaker successor model; the historical ledger supplies provenance, not the only route to understanding the code.

## Documents

- `docs/DESIGN-SPEC.md` — the authority for this repository.
- `docs/MODS-MAP.md` — every URL in the mods `references.md`, classified.
- `docs/SHAPERS.md` — each shaper with the paper it is developed from.
- `docs/APPEARANCE.md` — the OpenPBR + NPR appearance contract, 4×4×4 and 256×.
- `docs/LEDGER.md` — the decisions taken in this repository, appended, never rewritten.

## Build

```bash
cargo test 
```

Two things about this repository's build that are environment facts rather than
taste, and both are recorded in `docs/LEDGER.md` (C-32):

* **The host is MSVC.** `wgpu` needs the `windows-*` crates, which need import
  libraries that the GNU toolchain's bundled `dlltool` cannot build on a machine
  with no MinGW assembler. The compiler revision is unchanged by the switch.
* **The build tree is this repository's own `target/`**, the cargo default: there is no
  `target-dir` override in `.cargo/config.toml` any more, and the file's comment carries the
  measurements for why. In short, the tree used to sit on `F:` for *space*, and `F:` turned
  out to be this machine's only mechanical disk while `F:`'s space advantage had already
  gone. The source is on `C:`, an SSD, so the build is on `C:` with it — one volume, the
  fast one, and `target/` is the first line of `.gitignore`, so a build leaves `git status`
  clean. `kache`'s store lives on `C:` too, which is not tidiness — same-volume cache hits
  restore as hardlinks instead of copies.

YOU ARE NOT AUTHORIZED TO SKIP ANY TESTS


Run the window. `SHPLOINGUS_WORKERS` sets the worker count, which is the knob the
on-screen determinism panel is a claim about — set it to `1` and to `15` and read
the same resident digest out of the panel.

```bash
cargo run -p shploingus
```

> ### **Never test the client invisibly.**
>
> Any agent, contributor or script that exercises the client runs it **with the
> window drawn**, so a human can watch it. Never `nodraw`, never a hidden or
> detached window, never a headless substitute for looking. The offscreen
> `Renderer` exists to *measure* pixels and it is good at that, but it is not a
> substitute for the thing being seen: the black-screen defect recorded in
> `docs/LEDGER.md` C-38 survived a full green test suite and was found by a person
> looking at the window. A test that hides the window removes the only observer
> that found it. Ruled by the user, and recorded in C-40.
