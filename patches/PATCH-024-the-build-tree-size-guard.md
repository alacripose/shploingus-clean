---
patch_id: PATCH-024
title: The build tree's size guard — the door that stops a full volume from killing a build
status: applied
operation: add
target: shploingus/space-guard
base_ref: shploingus-clean@main
base_digest: sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1
dependencies: [PATCH-020, PATCH-022]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**The user's report, twice: *"you keep running out of space because the target folder is now 7
gigabytes large."*** The measurement behind it, taken 2026-10-10: `C:` was **232 MB free of 465 GB**
— 100 % full — with `target/` at **6.9 GB** (2.5 GB of compiled example binaries, 2.3 GB of
incremental state, 0.4 GB of debug symbols) and the kache store at **9.6 GiB** beside it. Every one
of those gigabytes is regenerable, and every one was being kept, because **nothing in the project
had an opinion about how large its own build tree may get.** A build does not fail because a tree is
large; it fails because nobody decided.

This patch is that decision, in one place, asked on every build the gate runs.

## 1. `voxel_package::space` — the measurement, the decision, and the trim

A module, not a script, on the same ruling `bloat.rs` records: a finding that *"can only be read in
a terminal"* is incomplete, so the guard lives in the pass the build already runs.

**It trims four parts, cheapest to lose first, and stops as soon as the volume has its headroom
back:**

| part | what it is | cost of losing it | measured here |
|---|---|---|---|
| `doc` | `target/doc` | a `cargo doc` | 3.0 MB |
| `examples` | a profile's `examples/` | the examples rebuild, only when one is asked for | 104 MB |
| `incremental` | a profile's `incremental*` | the next build of those crates is a full rebuild | 213 MB |
| `symbols` | `deps/*.pdb` | line numbers in a backtrace until that unit is relinked | 79 MB |

**The `symbols` cost is measured, not assumed, and it is the reason the part is on the list.** The
obvious worry is that a `.pdb` is a tracked output and deleting one forces a relink of everything
that produced symbols — which would make the part a trap. It is not: a 21 MB `.pdb` was deleted from
a settled tree and `cargo build` answered `Finished in 0.33s` with no relink, and the same test in
the release profile answered `Finished in 0.84s`. Cargo does not treat a `.pdb` as a freshness
input, and it does not put the file back until that unit is next built. So the part is real: 0.4 GB
of the tree buys nothing tonight.

**Everything else is a keep list, each entry with its reason, and every entry reported:** `debug/deps`
(reusable compiled artefacts: removing them is a full rebuild, which is the cost the guard exists to
avoid), `release/` (the profile that ships), `.fingerprint/` (cargo's own freshness record — the
fewest bytes in the tree, 1.4 MB across 4 009 files, and the most expensive thing in it to lose),
`build/*/out` (sources other crates include through `OUT_DIR`). Deleting any of them is `cargo
clean`, which is exactly what this guard is not.

## 2. The three thresholds, each derived from a measurement

* **`FLOOR` = 8 GiB.** The observed peak tree is 6.9 GB, so 8 GiB is the smallest headroom that
  holds one whole tree; below it the next large build is a coin flip. Restatable per machine with
  `SHPLOINGUS_SPACE_FLOOR=12GiB`; a value that does not parse is **refused and reported**, and the
  default stands. There is no switch that turns the guard off, on purpose: its only trigger is *the
  volume is filling*, so the one thing an off switch would be used for is a build that cannot finish.
* **`TREE_BUDGET` = 4 GiB**, the trigger used **when the volume cannot be read**. Measured on
  2026-10-10 the irreducible tree is about 1.6 GB (`deps` 0.49 GB debug + 0.92 GB release, `build/out`
  0.15 GB, `.fingerprint` 0.01 GB), so 4 GiB is one regrowth of the trimmable parts on top of it —
  high enough that the guard does not trim after every build, low enough that it acts before the
  parts are again measured in gigabytes.
* **`MINIMUM` = 1 GiB**, the point below which the gate **refuses**. The death that prompted this was
  at 232 MB with a relink in flight, so a gigabyte is named as the smallest headroom this tree has
  ever been seen to work in. A guess in this direction costs a build that could have finished, so the
  refusal line is deliberately the lowest of the three — the floor trims, only the minimum refuses.

**The trim is a byte budget, not a loop of probes.** A volume reading costs a process, so it is taken
once before the pass and once after: `floor - free` is the number of bytes the volume needs, and the
guard stops the moment the parts it has taken add up to it. What was freed is the **difference**
between what the scan measured and what is still at the path after the removal — a file another
process holds open is not credited to the guard as bytes it did not win. The second probe reports the
outcome, and when the volume cannot be read at all the guard keeps the reading it had rather than
inventing one from the bytes it removed.

## 3. The wiring, and where it runs

`crates/shploingus/build.rs` runs the guard **before it compiles anything**, so a volume that is
filling is answered before the gate adds to it rather than after cargo has died in it; the section is
printed with the rest of the evidence, and the refusal — the guard's **only** fatal branch — fires
after the report is written, like the conformance door beside it. A full disk is checked first of the
two doors: it is the one refusal that makes every other answer unactionable.

`cargo space` (`crates/voxel-package/examples/space-guard.rs`, alias in `.cargo/config.toml`) is the
same pass on its own, on the crate whose dependencies are headless — which is what makes it usable
when the GPU stack cannot build. Its exit code is the verdict (`0` nothing refused, `1` below the
minimum, `2` usage).

## 4. The report, and what it refuses to say

`key<TAB>value`, like every other gate line, with the project's own rules kept: an absent part prints
**the places it searched**, never a row of zeroes; a volume that could not be read prints
`not-measured` with **each probe's own words**; the moment every number belongs to is stated, so a
reader cannot think the removed bytes are still on disk; symlinks are skipped **and named**, never
followed; the levers the guard will not pull are named with the command that pulls each (`kache gc`;
`logs/`), because it will not delete another tool's cache — or the project's evidence — to make its
own number look good.

**One bug found by reading the guard's own output on this tree**, and it is the kind this project's
gates exist to catch: the first version collected only the `.pdb` files it *found*, so a part whose
files had just been taken printed `space-part symbols empty searched ` — a **blank** list, which
reads as *looked nowhere*. The search sites are now the authority and a part with no container at all
says `none: no profile directory under …`. Held by
`space::tests::a_part_trimmed_to_nothing_still_says_where_it_looked`.

## 5. The correction to `.cargo/config.toml`

The file claimed a same-volume cache hit *"costs zero extra bytes and no copy at all"*. **kache's own
advice line says otherwise on this machine:** *"cache hits are restored by COPY — the cache and build
tree do not share storage blocks, roughly doubling disk for cached content … set `[cache]
windows_hardlink = true`"*. That switch is **not** set, and the reason is this patch: kache warns it
is only safe *"if your build never deletes or rewrites an object output"*, and the size guard deletes
object outputs by design. Enabling zero-copy dedup and keeping a guard that trims the tree are in
direct tension, so the config now records the measurement instead of the claim, and the tension
instead of the silence.

# Acceptance

- `voxel_package::space` trims **only** paths it constructed by joining the build tree's own root,
  refuses any path outside it by name, never follows a symlink, and never removes a path it could not
  measure — each of those held by its own test.
- The guard trims to a **byte budget** derived from one volume reading and reports what it freed as a
  measured difference, not as a sum of what it intended to free.
- `cargo check -p shploingus` runs it before compiling anything, prints its section in the gate report,
  and refuses only below `MINIMUM`; the guard's four thresholds are asserted ordered at compile time.
- A part that holds nothing prints **the places it searched**, and a part with no container says so —
  no line of the report is a blank search list.

# Evidence

* `cargo test -p voxel-package --lib space::` — **21 passed**, covering the refusal set: every path
  the guard removes is inside the tree (`inside`, asserted over the scanner, not the trimmer); a path
  outside it is refused rather than removed; a symlink is skipped and named; the keep list is never a
  candidate and every entry has a reason; a trim that cannot meet its budget takes only what it may
  and leaves `deps`, `.fingerprint` and `build/out` standing; an empty part renders its search sites
  and never a zero; an unreadable volume falls back to the tree budget **and says so**; a volume below
  the minimum gives a refusal that names the keep list and both levers, while a volume above the
  minimum is not a refusal even below the floor.
* `cargo test -p voxel-package --lib` — **312 passed, 0 failed, 1 ignored** (the whole crate, not
  only this module).
* **On the real tree, through the interface a person uses:** `cargo space --floor 20` (the volume had
  11.7 GiB, so the trigger fired) reported `space-guard short`, took `doc`, `release/examples`,
  `debug/incremental.kache-auto` and ten `deps/*.pdb` files, and measured `space-freed 369692581` —
  **352 MiB** — with `space-shortfall 9415733248` and exit code **0**, because 11.7 GiB is below a
  20 GiB floor but far above the 1 GiB minimum. Asked again at the default floor it answered
  `space-guard ok` / `space-trigger room` and removed nothing.
* `cargo check -p shploingus` — **exit 0**, with the guard's section in
  `target/debug/build/shploingus-*/out/alpha-shell.gate` (726 lines) and the conformance door
  unchanged: `conformance active`, `refused 12`, `accepted 12`, `rejected 0`, `unchecked 0`.

# Rollback

The patch's own files are three: `crates/voxel-package/src/space.rs`, its example runner, and the
`cargo space` alias, plus the wiring in [build.rs](crates/shploingus/build.rs) — deleting them restores
the tree's bytes. **The guard's removals are not part of the rollback**: what it trims is regenerable
by definition, and a trimmed example, incremental tree, doc or `.pdb` is rebuilt by cargo at the cost
printed beside the removal. The pin above makes the reading stale rather than wrong once the record
changes, which is the same rule this tree applies to every proof.

# Not done, and named

- **A cold tree can still die before any build script runs.** The guard runs when the gate runs, so a
  machine whose `target/` is empty and whose volume is already full fails while compiling the crates
  *before* `shploingus` — cargo has no hook earlier than a build script, and the earliest one
  (`voxel-model`) cannot depend on the crate that owns this module. The practical answer is that the
  guard keeps the tree small *because the gate runs often*; the honest limit is that it is not a
  pre-flight check, and the escape hatch (`cargo space`) itself needs to compile a headless crate.
- **The removal happens while the build runs.** Cargo treats a missing part as absent and produces it
  again, which is measured above for `.pdb` files and documented for incremental state; the one
  corner not measured is a *concurrently running* `rustc` whose own incremental session directory is
  deleted under it. That risk is taken only when the alternative is a certain failure, and it is
  stated in the report's own `space-guard-is-not` line rather than left implied.
- **The kache store (9.6 GiB) and `logs/` (413 MB) are named and not trimmed.** The first is another
  tool's cache and the lever is `kache gc`; the second is retained evidence and deleting it is a human
  decision. Both are printed in the report as levers, so a reader whose volume is still short knows
  where the bytes are — but the guard will not pull either.
- **`kache`'s copy-restore is recorded, not repaired.** Zero-copy dedup would roughly halve what the
  cache costs (see §5); it is off because it is unsafe beside a guard that deletes outputs. A repair
  worth doing later — if the guard's trimming is ever proven never to rewrite an output in place —
  would be to set `windows_hardlink` and measure the difference.
