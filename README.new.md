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
| `voxel-model` | signed coordinates, the 4×4×4 subvoxel model, 256× material detail, identity, refusal, and the decision layer's vocabulary |
| `voxel-noise` | the noise primitives every shaper is built from, each from its paper |
| `voxel-shapers` | the terrain shapers: tectonics, geology, terrain, erosion, hydrology, climate, weather, caves, substrate, ecology, delta, transition |
| `voxel-mesh` | sparse voxel octree + greedy visible mesh |
| `voxel-stream` | a `WorldSource` the mesher can read, the four horizons, the resident set |
| `voxel-instrument` | the one sensor concept, the min/max band, the weighing gate, the gauge wall |
| `voxel-appearance` | OpenPBR inputs derived from block truth, the four Source 2 terms over them, the 256× detail lattice addressed |
| `voxel-work` | the worker pool, the box-shaped streaming window, and the memory budget — the only crate that spawns a thread, and it depends on nothing outside the workspace |
| `voxel-client` | the GPU: meshes and a camera in, pixels out, **offscreen** — with the shader held to `voxel-appearance` by a test that compares the two on the same surface, and the HUD (`glyphon` text plus a rounded-rect panel pipeline) so a frame is drawable *and* measurable without a window |
| `voxel-ui` | the design system — Material Design 1 with the Human Interface Guidelines overlay, every constant labelled *MD1 rule* / *MD1 example* / *platform translation* / *project extension*, and **zero dependencies**, because a design system is a set of statements about numbers and statements about numbers can be checked without a display |
| `voxel-package` | content intake: a package is admitted by validation, never by optimism. Integrity and identity are separate types because a checksum is not a signature. The back-compiler (`compiler::emit`) and the decision seam (`decision::Decider`) live here |
| `shploingus` | the window — the only crate with a windowing dependency, and the only place a person sees the world |

## The two rules that govern everything

1. **Refuse, never default.** A value that cannot be validated is refused and
   named, never replaced by a plausible one. A plausible substitute is
   indistinguishable from a correct one until much later — and by then it has
   been written into a world.
2. **Determinism is a contract.** The world is a pure function of
   `(seed, tick, declared period)`. Two runs over the same inputs are
   byte-identical, and that is a test, not an aspiration.

## The architecture contract (Structurizr DSL)

**`architecture/workspace.dsl` is the system operating contract.** It is the hard
boundary no agent may violate, and it exists because agents that "flub up" a
project almost always fail the same way: context-window collapse — lost
boundaries, over-engineering a small feature, breaking upstream contracts,
inventing dependencies. That failure is not hypothetical; it is what a
governance-stack session did to this tree before this section existed.

The contract is enforced by four rules (`docs/FUTURE_PLANS_FOR_ANY_CURIOUS_MODELS.md`):

1. **Zero-erosion rule.** No software system, crate, store, or third-party
   connection is introduced unless the `model {}` block of `workspace.dsl`
   declares it.
2. **Interface contracts.** A change to a communication path requires the
   relationship arrow to exist in the DSL first. Before adding a dependency,
   verify the arrow; if the arrow should exist, change the DSL *first* and say so.
3. **Flub mitigation protocol.** On an implementation error, do not rewrite
   adjacent modules or inject global helpers. Check whether the design has
   drifted from the container blocks.
4. **Pre-flight block.** Every task response begins with:

   ```
   Target Container: [e.g., voxel-package / Back-compiler]
   Permitted Upstream/Downstream: [e.g., chain -> compiler -> decision -> queue]
   Drift Evaluation: [the change requires no structural drift]
   ```

### The model

```structurizr
workspace "Shploingus" "ticket -> back-compiler -> recipe -> code, with the decision layer inside" {
    model {
        person = person "The player" "files plain prose tickets at the console"

        chain = container "The chain" "chains/workspace.chain — the one append-only record" "chain"
        decision = container "The decision layer" "laya + decision::Decider — decides every stage, full axis suite" "Rust / voxel-model"
        compiler = container "Back-compiler" "voxel-package::compiler — Solution::for_ticket -> render -> compile" "Rust"
        packages = container "Packages" "packages/ — the loader's search path IS the compiler's output root" "shpkg"
        queue = container "Notification queue" "logs/laya-decisions/ — musli-wire records, sha256-verified, streamed" "api layer"
        game = container "The game" "shploingus + voxel-client — the window and the shell" "Rust / wgpu"

        person -> chain "files tickets through the handler, never by hand"
        chain -> compiler "tickets read back"
        compiler -> decision "every stage decided"
        decision -> queue "one notification per decision"
        compiler -> packages "emitted artifacts, sha256 attached"
        packages -> game "loaded and verified before use"
        queue -> game "the notifications window streams it"
    }
}
```

The arrows are the contract. Three paths are forbidden outright, and each one is
a mistake this tree has already lived through:

* a **second record** beside the chain (a scratch ledger that drifts);
* a **queue format outside the declared codec** (a hand-rolled line format is an
  A08 finding the gate convicts);
* a **decision system that does not sit on the compiler's path** — the judge is a
  layer of the stack (`docs/LEDGER.md` C-82.10, C-83, C-84), never a parallel
  auditor that reports on the stack from outside it.

### The audit loop

Before anything lands, a separate auditor maps the `git diff` against the
container definitions in `architecture/workspace.dsl`: *did the change introduce
side-effects, implicit modules, or global state that bypass the declared
boundaries?* If yes — reject and roll back. A diff that skipped this check is
**unverified**, whatever its tests say. `docker run -it --rm -p 8080:8080
-v $(pwd)/architecture:/usr/local/structurizr structurizr/lite` renders the model
live while you work.

## Operating rules for agents

Each rule is written in this tree's ledger because an agent broke it. The event
numbers are in the session ledger (`hypergrilling-ledger.jsonl`).

1. **Read the record before writing.** A file that exists is evidence; clobbering
   it is a regression, and "I did not look" is not a defence. (strikes 1–2.)
2. **Never invent a surface the record already has.** Formats, names, and paths
   come from `docs/LEDGER.md` and the evidence folders — not from what is
   convenient to parse. A `.jsonl` queue was invented once; the decided stack is
   `musli`/`musli-wire` for metadata records with `identity::Digest` sha256 over
   the artifacts and a directory the client watches
   (`docs/evidence/screenshots/IMG_3318/3320/3365`).
3. **Name the project's surface, not the crate.** The sha256 of a thing is
   `identity::Digest::of` in this tree. Reaching for a hash crate's raw type in
   new code re-invents a surface the record already names.
4. **The decision layer is inside the stack.** `laya` decides across the
   framework, the game's systems, and the compiler itself — the full axis suite,
   every decision notified to the api layer. A judge built beside the stack is
   the exact defect C-82.10 corrected.
5. **Dependencies may change.** Old rulings are history, not walls — the ledger
   is append-only and later decisions supersede earlier ones. `--offline` was
   never a decision; up-to-date packages matter.
6. **Artifacts live in the project.** Gate logs are `logs/gate-*.log`, the queue
   is `logs/laya-decisions/`, test scratch is `logs/test-workspaces/`. Nothing is
   written to a temp directory: a test over a temporary directory is evidence
   about the temporary directory (C-73).
7. **Stream.** Read content in rather than grepping around it; run long work
   detached and poll its log. Long foreground timeouts waste the person's time
   and destroy other processes' output.
8. **A grant names its object exactly.** Do the one thing authorized. A second
   loop over retired work is a regression even when the guard prevents the
   damage.
9. **Documentation lives in documentation.** Prose packed into code comments is
   what makes code fragile to edit. Code in code; docs in docs.
10. **Unruled questions come back as questions.** Deriving a decision the record
    has not made — window derivation from prose tickets, for instance — is not
    the agent's to decide.

## Continuing development

Start with `docs/HANDOFF.md`: current implementation, verified commands, the next
concrete task, and remaining alpha gates. It is deliberately short enough for a
weaker successor model; the historical ledger supplies provenance, not the only
route to understanding the code. Before touching the governance stack, read
`docs/FUTURE_PLANS_FOR_ANY_CURIOUS_MODELS.md` and the C-82/C-83/C-84 entries in
`docs/LEDGER.md`.

## Documents

- `docs/DESIGN-SPEC.md` — the authority for this repository.
- `docs/MODS-MAP.md` — every URL in the mods `references.md`, classified.
- `docs/SHAPERS.md` — each shaper with the paper it is developed from.
- `docs/APPEARANCE.md` — the OpenPBR + NPR appearance contract, 4×4×4 and 256×.
- `docs/LEDGER.md` — the decisions taken in this repository, appended, never rewritten.
- `docs/FUTURE_PLANS_FOR_ANY_CURIOUS_MODELS.md` — the Structurizr contract plan.

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

**The laya gate runs at `cargo test` time, and all code fails it by default.**
An artifact clears only with proof of work: a claim on the record plus evidence
with an origin. Every decision produces exactly one notification in
`logs/laya-decisions/` — the api layer carries the whole decision, findings and
grades included — and the oracle's current word is `logs/laya-report.txt`, where
hand-rolled wire formats are named with the line that triggered them.

The laya juge/decisioner/oracle is a key integrated system and is there to make your job 
as an agent easier, not harder. It is supposed to take the user filed tickets in  the game cli, 
and digest it through the system to make the recipes you use to code with, which is why we are now using structurizr.

```bash
cargo test --workspace --nocapture
```

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
