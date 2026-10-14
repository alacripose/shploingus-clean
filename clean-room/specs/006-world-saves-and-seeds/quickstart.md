# Phase 1 — Quickstart: validating U9

**Feature**: `006-world-saves-and-seeds` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Read this first.** U9 is **built and measured** (2026-10-12), in both of the forms the plan named:
the exact length-framed encoding with its trailer, the parser, the five per-domain stream seeds carried
and checked in both directions, the ruleset identity, the record carried in its own wire form and
declared by head and length, six named refusals, and a real file round trip; and, since the same date's
later pass (§28), the **document form** — a world written at and read back from a declared reference
(`path + digest`) through **U6's content gate**, here a hard prerequisite, reached through the one tool
table's own **`save`/`load` rows** (its seventh and eighth). What is still **owed and named**, and printed
by the binary as owed: a save of a **resident set**, **compression**, and a **versioned migration path**.

## Prerequisites

```bash
cd clean-room                # its own Cargo workspace, beside the legacy tree
cargo --version              # the toolchain the workspace pins
```

No network is required: the crate depends only on `sha2`, already resolved locally (`--offline`).

## Scenario 1 — a written world reads back as the same world (SC-001, SC-006)

```bash
cargo run --offline
# measured 2026-10-12 (the binary takes no arguments; the world is the reference declaration's):
#   U9 save:          465 bytes at "C:\\Users\\ala\\AppData\\Local\\Temp\\shploingus-u9.save"
#                     (the file and the re-encoding are the bytes written = true)
#   U9 identity:      seed=1592639215 period=1s tick=3  ruleset 7 revision 3
#   U9 record:        2 event(s), head 6d0ae0913d918d986e0c5ab51049a8f4f707694f3398db6138cecffe278acb23
#                     — declared and carried agree = true
#   U9 loaded truth:  32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539
#                     (the live world at this tick: 32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539
#                     — equal = true, and the previous tick differs = true)
#   U9 rebuilt at 1 worker(s): c331cf554a47e777254d32b673079d7abc0eca624da2e4afd705160a02a5c74b
#   U9 rebuilt at 2 worker(s): c331cf554a47e777254d32b673079d7abc0eca624da2e4afd705160a02a5c74b
#   U9 rebuilt at 4 worker(s): c331cf554a47e777254d32b673079d7abc0eca624da2e4afd705160a02a5c74b
#   U9 rebuilt at 8 worker(s): c331cf554a47e777254d32b673079d7abc0eca624da2e4afd705160a02a5c74b
#   U9 worker knob:   4 digest(s) at 1/2/4/8 workers, one value = true; rebuilt == ticked = true
#   Clean room OK: every built system's acceptance bar is measured and passing.
```

**Passes when**: the loaded world's truth digest **is** the live world's, the record it carries is the
record it declares, and one generation digest covers 1, 2, 4 and 8 workers. **Measured:** all three
hold. `465 bytes` is the whole written world — identity, five stream seeds, the ruleset identity and
two events — and the world rebuilt from it generates `c331cf55…` at every worker count, which is U3's
instrument (`460214b1…2fc6`) applied to the identity a **save** carries.

## Scenario 2 — the seed systems are five, not one dial renamed (SC-002, SC-004)

```bash
cargo run --offline | grep '^U9 stream'
# measured 2026-10-12:
#   U9 stream seed:   terrain   110187191988996982 (= the world's own: true)
#   U9 stream seed:   geology   85828532078282826 (= the world's own: true)
#   U9 stream seed:   climate   6158460919897384818 (= the world's own: true)
#   U9 stream seed:   hydrology 17877078227061120690 (= the world's own: true)
#   U9 stream seed:   detail    164424443644523746 (= the world's own: true)
#   U9 streams:       5 of 5 declared, each the world's own = true
```

**Passes when**: the save carries **one seed per declared `Domain`**, each the world's own, and a load
checks every one. **Measured:** five streams, five distinct seeds, all five equal to
`World::stream_seed(domain)` for the identity the save names. A save that omits a stream, names one this
build does not declare, or carries a seed this world does not derive is refused **by name** — pinned in
`src/save.rs` by `a_save_missing_a_declared_stream_is_refused_by_name`,
`a_stream_the_receiver_does_not_declare_is_refused_by_name`,
`a_stream_seed_that_is_not_the_worlds_own_is_refused_with_both_values` and
`every_fault_in_a_save_is_collected_not_just_the_first` (four faults in one pass).

The tick is not the seed: two saves at consecutive ticks share the seed and the period and differ only
in the tick (C-96.1) — pinned by `consecutive_ticks_share_the_seed_and_differ_in_the_tick` and by
`consecutive_ticks_share_the_seed_and_differ_in_the_tick` in `tests/u9_saves.rs`.

## Scenario 3 — a save that cannot be trusted is refused by name (SC-003, SC-005)

```bash
cargo run --offline | grep '^U9 refusals'
# measured 2026-10-12:
#   U9 refusals:      not_a_save = true, save_truncated = true, save_tampered = true,
#                     identity_mismatch (another ruleset) = true
#   U9 refusals:      seed_undeclared and record_diverged are pinned by the crate's own writer tests
#                     (src/save.rs) — the public writer will not emit a save it would refuse
#   U9 owed:          a save of a **resident set** (a save is the record, not the residency — F1),
#                     compression, and a versioned migration path
```

**Passes when**: each way a save can be wrong has its **own** name, and no refusal yields a world.
**Measured:** four of the six are reached through the public surface above; the other two —
`seed_undeclared` and `record_diverged` — need bytes a caller cannot write through this build's public
writer (a save the writer would refuse), so they are pinned by the crate's own writer tests and named
here rather than quietly omitted. `Loaded` is returned only when every check passed, so there is no
partially loaded world for a caller to trip over.

## Scenario 4 — the document form: a world written at and read from a declared reference (SC-001, SC-004)

```bash
cargo run --offline | grep '^U9 document'
# measured 2026-10-12 (§28):
#   U9 document:      `save` wrote 465 byte(s) at the-world.save
#                     (98fc98f1722f942c17f3c72f29dcfa96d35b02e12e00b74e6c6c4e7f110e62dc),
#                     seed=1592639215 period=1s tick=3 ruleset 7 revision 3 — its receipt verifies
#                     (claims re-established from the request, not believed) = true
#   U9 document:      `load` read them back by reference: truth=32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539
#                     — the live world's 32b3c1f8db8202e202b65cbe88f8a0f2caa6df2863cc5f7d9649cd5333395539
#                     (equal = true); the write and the read are one value = true;
#                     the library's own door reads the same document = true
#   U9 document refusals: an absent document [source_absent], bytes altered inside the save's own trailer
#                     [save_tampered], and a document that is not the one the reference names
#                     [source_digest_mismatch] — each by its own name = true, each recorded = true,
#                     and the record still verifies = true
```

**Passes when**: the world written down is the world read back, the reference is a reference (`path +
digest`, never bytes), the gate is a **hard** prerequisite of the read, and each way the document can be
wrong refuses **by its own name** *and* is recorded. **Measured:** all four hold. The 465 bytes are the
same world Scenario 1 writes and reads in-process, and the `load` row's truth digest is the live world's
— so the two forms are one world reached two ways, not two similar numbers. `save` and `load` are rows of
the one tool table (`bridge::TOOLS`, closed at **eight** verbs), and `Save::read_document` reads through
`Intake::read`: the module has no `fs::read`, which is why "consumed in place" is the shape of the code.
The three refusals are the two the gate owns (an absent document, and bytes that moved) and the one U9
owns (bytes that are the declared ones and disagree with their own trailer); all three land on the
door's record, which still verifies.

## Scenario 5 — the checks that decide the pass (SC-007)

```bash
cd clean-room
cargo test --offline
# measured 2026-10-12 (final bytes, §28):
#   276 lib tests passed, 0 failed    (20 of them are `save::tests::…`)
#   2   u4_geometry   passed, 0 failed
#   6   u5_measure    passed, 0 failed
#   7   u7_decision   passed, 0 failed
#   7   u10_gameplay  passed, 0 failed
#   5   u6_admit      passed, 0 failed
#   7   u6_bridge     passed, 0 failed
#   6   u6_intake     passed, 0 failed
#   6   u6_transport  passed, 0 failed
#   6   u9_saves      passed, 0 failed
#   = 338 passed, 0 failed    ← §28's paste, as that pass measured it
#   *(corrected 2026-10-12, §29: the suite is at **351** passed — **283** lib + 6 `u6_policy` — and
#   `cargo run --offline` prints **498** lines; see ../../../docs/baselines/u5-voices/post-stdout.txt)*
#   *(corrected again 2026-10-13, §31–§32: §31 moved the suite to **365** and this pass to **374**
#   (300 lib + 74 integration) at **504** stdout lines — see
#   ../../../docs/baselines/u12-first-mob/final-stdout.txt; the figures above are §29's own dated reading)*
#   *(and §33's repair, 2026-10-13: the suite stays **374** (300 lib + 74 integration) and stdout stays
#   **504** lines, but two `U12` lines are re-worded — the world read as one static evaluation at `Tick(0)`
#   and the declared fact's own name in place of a count — so the current capture pair is
#   ../../../docs/baselines/u12-repair/pre-stdout.txt and final-stdout.txt, digest `5f4d5c80…`; §32's own
#   pair keeps its dated digests)
#   *(and §34, 2026-10-13: the suite is at **381** passed / 0 failed (**304** lib + 77 integration) and stdout
#   at **507** lines — the registry's `[U12]` row and the `U12 first mob:` line re-worded plus three `U12
#   stepping` lines — with the current pair at ../../../docs/baselines/u12-stepping/pre-stdout.txt (504 lines,
#   `5f4d5c80…`) and final-stdout.txt (507 lines, `b0942531…`); the figures above are §33's own dated
#   reading)*
#   *(and §35, 2026-10-13: the suite is at **383** passed / 0 failed (**305** lib + 78 integration) and stdout
#   at **507** lines with three lines re-worded — the plane now steps **to rest** (`SETTLE_TICKS = 200`), the
#   settled figures print beside the tick-8 transient, the being's need prints as its own value, and the water
#   has one authority. The current pair is ../../../docs/baselines/u12-settled/pre-stdout.txt (507 lines,
#   `b0942531…` — §34's final pair, inherited by digest) and final-stdout.txt (507 lines, `d14588a5…`); §35)**
cargo clippy --offline --all-targets -- -D warnings
#   Finished `dev` profile … — exit 0, no diagnostics
cargo run --offline > run-1.txt; cargo run --offline > run-2.txt
sha256sum run-1.txt run-2.txt
#   both exit 0; 487 lines of stdout, byte-identical
#   (the U9 document form added twenty lines and removed none — see
#   ../../../docs/baselines/u9-pre-stdout.txt and u9-post-stdout.txt)
cargo tree --offline | grep -c '^├──\|^└──'
#   1 — `sha2` remains the crate's only dependency
```

**Boundary (P7), re-measured:** `find ../crates -newermt` returns nothing for this pass's window — no
file under `crates/` was read or written while writing this slice, and no legacy save format was
carried.

## How to read a failure

- **A digest that moved between worker counts** means the world function gained a worker-dependent
  input: U3's invariant is broken, not U9's.
- **`save_tampered` where a truncated file was expected** means the file's own lengths still add up —
  the layout accounts for every byte, so the trailer is the first thing that disagrees.
- **`identity_mismatch` naming a stream** is the five-seeds check doing its job: the bytes are intact
  and coherent, and they are simply not this world's.
