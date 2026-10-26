# PATCH-045 — the balance census

The user's sentence, 2026-10-21, is this pass's specification:

> *"the newly defined axis crates system needs an extra crate for verifying the balance of cpu code
> vs gpu code and how many items in cpu code have no gpu renderer"*

`voxel-axis-balance` is that crate. It is the seventh in `crates/axis`, it is headless, and it
opens nothing for writing.

The act is at `crates/axis/balance/src/lib.rs:1`; the check is `cargo test -p voxel-axis-balance`; the witness is `crates/axis/balance/src/lib.rs`.

# Two measurements, and they are not the same kind

| measurement | produced from | what it answers |
|---|---|---|
| `Measure` | the tree, walked by `measure.rs` | how many files, lines and bytes of Rust and of WGSL each crate holds |
| `Item` | `inventory.rs`, **declared** | which public visual item exists, and which renderer each one reaches |

**The first is measured; the second is declared, and the crate says so in its first paragraph.** A
file walk cannot decide whether a construct is drawn: a `pub struct` says nothing about a frame,
and a grep for a type in the renderer counts a comment as a renderer. So the inventory is a table,
and every row is then held to the tree:

| a row... | is refused as | because |
|---|---|---|
| names a site the tree does not have | `FabricatedFeature` | a row that names a witness it does not have is worse than no row |
| names a declaration its site no longer contains | `FabricatedFeature` | the row has outlived the thing it describes |
| names a shader entry point its file does not define | `MissingStage` | a declared stage that is absent is a blocker, never an omission |
| names a Rust renderer its file does not contain | `MissingStage` | the renderer the row means is gone |
| names no renderer | **counted as unrendered** | this is the census's finding, not a failed check |

The inventory is complete for the closed sets it names: the shell's element vocabulary is three
variants, and a test reads them out of `voxel-ui/src/lib.rs` and asserts each has a row, so a
fourth element cannot be added without being counted.

# The balance, written as operands

The ratio is Rust lines per line of WGSL, per crate and for the workspace. It is written with
both operands beside it and it is **not a score**: a test is not a shader, and a crate whose tests
are longer than its library is not mostly CPU. Each crate is measured over its `src/` only — the
code that ships, not the examples that demonstrate it.

# The finding

On the checked-in tree the inventory holds 20 items: 18 rendered, **2 with no renderer** —

| item | site | why it is counted |
|---|---|---|
| `desktop` | `crates/voxel-client/src/shell/desktop.rs` | the window-list model, exported and named in no draw path |
| `taskbar` | `crates/voxel-client/src/shell/taskbar.rs` | the bar's labels and clock, exported and named in no draw path |

Both are ported modules that the live mount does not call. They are **reported, not deleted**:
whether they are wired up or removed is a decision this crate is not entitled to make, and a
module that vanishes from a count is how a dead surface stays dead without anyone noticing.

# What it refuses

| the reading | the refusal |
|---|---|
| the root has no `Cargo.toml` | `NoReading` — a balance of nothing is not a balance of zero |
| a crate in the census has no directory | `MissingFeature` — the table says it is there and the tree disagrees |
| a crate directory with no Rust at all | zero, which is a measurement and not a pass; the report prints the zero |

`Report::verified` is false when any of those holds, and the report is still printed: the rows that
could not be read stay in the table with their zero, so a refusal reads as a refusal instead of a
missing line.

# Wiring

* `crates/axis/balance` joins the workspace and the census (`Crate::Balance`, 22 → 23 rows), so
  it is measured by the gate that measures every other crate.
* Its six census axes are all `active`: named, moduled, tested, refusing, claimed in
  `DESIGN-SPEC.md` §3.1, and documented (0 missing docs, with its pin in
  `conformance/missing-docs.txt`).
* Its prose budget is 0 plain comments; the crate is written in rustdoc.
* `voxel-axis-verify`'s Axis subject gains the crate, so the axis segment's cell is measured over
  it in the same reading as the other six.
* It is an **orphan library** by `census`'s second rule, and the list in
  `census.rs` says why: its answer is a reading for a person and the shell's console, not a
  service another crate imports. It leaves the list the day a surface consumes its lines.

# What it does not do

* It does not fail a build. The report is a value and `verified` is its own bit; a gate that
  decided what an unrendered item costs would be a policy, and the policy is not this crate's.
* It does not parse Rust or WGSL. Files are counted by extension and lines by newline; the
  inventory is text-matched against the declaration it names, which is the same standard
  `voxel-package`'s conformance files hold their own claims to.
* It does not depend on any UI crate, so the inventory cannot become a second place a window is
  declared.
