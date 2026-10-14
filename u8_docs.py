"""Cascade U8 into docs/CLEAN-ROOM-PLAN-2026-10-11.md, docs/LEDGER.md and clean-room/README.md."""
import io

def read(p):
    with io.open(p, "r", encoding="utf-8", newline="") as fh:
        return fh.read()

def write(p, t):
    with io.open(p, "w", encoding="utf-8", newline="") as fh:
        fh.write(t)

def eol(t):
    return "\r\n" if "\r\n" in t else "\n"

def patch(path, pairs):
    text = read(path)
    nl = eol(text)
    for old, new in pairs:
        old = old.replace("\n", nl)
        new = new.replace("\n", nl)
        assert text.count(old) == 1, (path, text.count(old), old[:60])
        text = text.replace(old, new, 1)
    write(path, text)
    print("patched", path)

# ---------------------------------------------------------------- the plan ----
plan = "docs/CLEAN-ROOM-PLAN-2026-10-11.md"

status_bullet = """* **status:** **built** (2026-10-12, §27). The interface below is landed in `clean-room/src/shell.rs`
  (**1406** lines): the declared `Recipe` (six windows, and the mount adds none it was not handed), the
  `Mount` pane tree, the shell/HUD **layer toggle** over one window set, the **one** pointer-ownership
  decision read once and passed **by value** into both the drawing and the grab, and the four-bucket
  **admitted constants table**. §5 row 7's check — *a visibly drawn frame; pointer ownership read once* —
  is met by a **100 × 30** drawn frame (digest `59509ab2…`), and the binary's new `shell` mode is the
  interactive form. Owed and named: undock, the splitter-drag resize, the console's five graded levels,
  the toast's final form, and the `gholtzap/lintapple` **P8** gate — which is **U13's**, not this
  slice's, and §27 states why.
* **slice:** **7** — row 7 of §5 (*written as `8`; corrected in §16.3*). It is last because it is the
  only place a person sees the world, and it is the acceptance surface for every system above it.
"""

old_slice = """* **slice:** **7** — row 7 of §5 (*written as `8`; corrected in §16.3*). It is last because it is the
  only place a person sees the world, and it is the acceptance surface for every system above it.
"""

row7_old = "| 7 | U8: the window, the mount, the design system | U8 | a **visibly** drawn frame; pointer ownership read once |\n"
row7_new = ("| 7 | U8: the window, the mount, the design system | U8 | a **visibly** drawn frame; pointer "
            "ownership read once — **built (2026-10-12, §27)**: a declared `Recipe` mounted as a pane tree, "
            "the shell/HUD layer toggle over one window set, one pointer-ownership decision read once and "
            "passed by value, a four-bucket admitted constants table (16 rows), and a **100 × 30** frame "
            "whose six panes read the registry, the reference world, its materials, the stage roster, a "
            "derived chunk's geometry and the light table; the binary's `shell` mode is the interactive "
            "check. Owed: undock, the splitter-drag resize, the console's five graded levels, the toast's "
            "final form, and the **P8** `lintapple` gate (U13's) |\n")

section27 = """

---

## 27. The sixteenth pass — U8, the shell: a visibly drawn frame, and one pointer decision

**What ran.** U8's spec set was created under `clean-room/specs/010-u8-shell/` (spec, plan, tasks,
quickstart, research, data-model, `contracts/u8-seams.md`, and the requirements checklist) from §2's U8
entry, §5's row 7, `docs/SHELL-PORT.md` §1/§4/§6 and `docs/UI-SPEC.md` §1–§5/§8/§9, and the slice was
implemented in one new module, `clean-room/src/shell.rs` (**1406** lines), with the three refusals it needs
added to the closed vocabulary and the binary's new `shell` mode.

**The bar, and what reached it.** §5 row 7's check is *a **visibly** drawn frame; pointer ownership read
once* — and §2's interface sentence is the same bar as obligations: *a `Mount`-shaped pane tree driven by a
declared `Recipe` (not a hard-coded app registry), the two layers' toggle, one pointer-ownership decision
read once and applied to both grab and visibility, and engine constants sourced from an admitted table
rather than literals.* Each is a measurement, not a claim:

* **A visibly drawn frame.** `shell::frame` draws into a character `Canvas` and prints it. The declared
  extent is **100 × 30** (`shell::declared_area`), the frame's digest is
  `59509ab207efe296dfb5eacec9536ae67766546476fa9d47743b5c604930d64e`, and a whole interactive session is
  kept at `docs/baselines/u8-shell-session.txt`. The binary's new **`shell` mode** draws the frame and reads
  one command per line (`split`/`close`/`focus`/`layer`/`grab`/`quit`), redrawing after each.
* **The mount, and no second registry.** `Mount::of(&Recipe)` mounts the recipe's **six** windows and adds
  none it was not handed; `the_recipe_is_the_only_window_list` holds it.
* **The two layers' toggle.** `Layer::{Shell, Hud}` over **one** window set; the HUD draws the two windows
  declared `pinnable` — the reference's own defect fixed (*"shaper UI hidden when esc menu hidden or f1
  toggled"*, `docs/UI-SPEC.md` §1).
* **One pointer-ownership decision, read once.** `Mount::read_pointer` computes the value once and both
  consumers take it **by value** — the drawing and the grab — so `visible` cannot disagree with `owner`; the
  signature, not a comment, is the mechanism.
* **Engine constants from an admitted table.** **16** rows across the four buckets (md1-rule **8**,
  md1-example **2**, platform-translation **1**, project-extension **5**); an unadmitted name refuses
  `constant_unadmitted`; the frame's own geometry reads the table and the bar prints its derivation
  (`bar 6 cells (48 dp ÷ 8 px) floor 5`), so no literal hides in the drawing.
* **The panes show real state, not a mock.** The six windows read the systems already built: the registry
  (`13` systems), the reference world (`seed=1592639215`, its truth and residency digests), its materials
  (`6` admitted), the stage roster (`15` stages), a derived chunk's geometry (`1031 columns → 144 quads`,
  merge factor `7.16`) and the light table (`1361.000 W/m²`).

**What moved, and by how much.** `RefusalKind::COUNT` moved **77 → 80** with this slice's three kinds
(`constant_unadmitted`, `pane_unknown`, `split_floor_breached`), cascaded into `tests/u5_measure.rs` by
name. The library tests moved **263 → 274** (eleven `shell` tests) and the total **323 → 334**. Stdout moved
**464 → 467**: three lines added (U8's frame, digest and constants lines) and **none removed**, with the
`[U8]` registry line changed **in place** from `absent` to `built`. Both captures are kept beside this
record — `docs/baselines/u8-pre-stdout.txt` (`sha256 8dbce319…`) and `docs/baselines/u8-post-stdout.txt` —
so the next audit has a real baseline to diff.

**Measured on the final bytes.** `cargo test --offline` — **334 passed / 0 failed** (274 library + 60 across
the nine integration suites); `cargo clippy --offline --all-targets -- -D warnings` exits 0; `cargo run
--offline` exits 0 at **467 lines of stdout**. Digests: `shell.rs`
`bb9b25f4bd2a33b7f2d2f8e2184c9b3bae39f54604c6a54109ec41f6c5fa8824`, `refusal.rs`
`f70369e9f9c2b8c6b9d1136b9e89e58560a5e10340461629f681a04b80f6cd4b`, `evidence.rs`
`31ef69ca5744c5f2ce8a907f30f61d4e8efb4c7818700482e9ed252ccbdc40bb`, `main.rs`
`24a30e205b06dabb219033020cf7765b97f0c26a6dc3fb95c09fa734e28ca045`, `systems.rs`
`75892ea55ebc622ac5a012129b90a6144c3e1e993371e3ea0495cd8f9de9eb6b`, `tests/u5_measure.rs`
`4c2205b7cba59442bc33d30b8c173e9733e7fd77d83211efd3fcaca05979b401`, `src/lib.rs`
`dc92774eeab8828bcdeb4c2ab5a712d5a94ebe05e897406ecfc040fff53450f9`.

**The `gholtzap/lintapple` gate is U13's, and this pass says so rather than deferring it silently.**
`clean-room/specs/001-presentation/plan.md` §3 carries the row **`P8 … WIP | NEEDS WORK`**, §5 row 13
defines **U13** as *"the shell completed, and its gholtzap/lintapple gate"*, and `systems.rs`'s own U13
entry names the gate as a wait. U8 is the shell; U13 is *the shell completed* plus the gate, and it is last
by §2's own reasoning (*"completed first, it would be judged against nothing"*). Taking the gate here would
duplicate U13's bar, so this slice does not — and what it contributes to that gate is the half the gate
checks, **every constant admitted**, which is the constants table. The statement is repeated in
`clean-room/specs/010-u8-shell/spec.md` (its own section), so the deferral is on the record in both places.

**What did not move.** No other row's verdict, citation or entry set; **U1's remainder**, **U6's
remainder**, **U11**, **U12** and **U13** stay exactly as they were; the **four U10 rulings**, **`mc-ar-mod`**,
the **`noul`/`score` calibration sets** and the **score instrument's rungs** stand **owed** and untouched;
no second parser, renderer or pane tree appeared — the fence is still read in one place, and `shell.rs` is
the clean room's only pane tree; and the audit's finding that `clean-room/` is untracked in version control
is unchanged, so no VCS provenance is claimed for this slice either.

**What remains owed after this slice.** U8's own remainder — **undock**, the **splitter-drag resize**, the
**console's five graded levels**, the **toast's final form** — and the **P8** gate (U13's). The plan's order
then continues at §5 row 8 (the Lane A/B/C measurement on the clean room itself).
"""

patch(plan, [
    (old_slice, status_bullet),
    (row7_old, row7_new),
])
text = read(plan)
assert not text.endswith("\n\n")
write(plan, text.rstrip("\r\n") + section27.replace("\n", eol(text)))
print("appended section 27")

# -------------------------------------------------------------- the ledger ----
ledger = "docs/LEDGER.md"
c188 = """
## C-188 — The shell, in the clean room: a visibly drawn frame, and one pointer decision (2026-10-12)

### 188.1 What this entry answers

`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 7 sets U8's bar in one line — *a **visibly** drawn frame;
pointer ownership read once* — and §2's U8 entry states it as obligations. This entry records the slice that
reached it, the three refusals it added, and where every number can be read back.

### 188.2 The window is data, and the mount adds nothing

`docs/UI-SPEC.md` §1: *"Windows are data the shell mounts."* So `Recipe::declared()` is a list of
`WindowSpec` values — an id (a stable identity, never a list position), a title, a lane, a content kind and
whether the window may live in the HUD — and `Mount::of` mounts **exactly** that list. The test
`the_recipe_is_the_only_window_list` is what makes *no hard-coded app registry* a check rather than a
promise: a mount that introduced a window of its own fails it. `shell.rs` is also the clean room's **only**
pane tree, which is `docs/SHELL-PORT.md` §4.2's own rule (*a second tree is the defect*).

### 188.3 The two layers are one window set

`docs/UI-SPEC.md` §1 records the reference's own defect — *"shaper UI hidden when esc menu hidden or f1
toggled"* — and its fix. `Layer::{Shell, Hud}` and `toggle_layer` implement the fix: the HUD draws the
windows declared `pinnable`, and the toggle **moves** them rather than rebuilding a second window set.

### 188.4 One pointer decision, read once — enforced by the signature

`Mount::read_pointer() -> Pointer { owner, visible }` is the single decision point. The value is handed
**by value** to `shell::frame` and to the event handling, so neither consumer *can* read it again, and
`visible` is computed inside that same read: a frame cannot show a glyph the shell does not own, nor own one
it does not show. When the layer is hidden, or nothing is grabbed, the world owns the pointer.

### 188.5 The constants, the panes, and the three refusals

The four buckets (`docs/UI-SPEC.md` §8) hold **16** rows — md1-rule 8, md1-example 2, platform-translation 1,
project-extension 5 — and the **extension** count is pinned by a test, because it is the bucket that can grow
silently. `constant(name)` is the one door and refuses `constant_unadmitted`. The six panes read systems
already built rather than mocks: the registry, the reference world's identity and digests, the material
registry, the stage roster, a derived chunk's geometry and the light table. This slice added two more kinds
to the closed vocabulary — `pane_unknown` (a pane is addressed by its own identity, never a list position)
and `split_floor_breached` (the 40 × 40 floor is refused, never clamped) — so the vocabulary moved **77 → 80**,
pinned in `tests/u5_measure.rs` by name.

### 188.6 Measured, and where the numbers are readable

**334 passed / 0 failed** on `cargo test --offline` (274 library + 60 across the nine suites); clippy
`--all-targets -- -D warnings` exits 0; `cargo run --offline` exits 0 at **467 lines of stdout** — three
added against the **464** before, and none removed. The frame is **100 × 30** with digest `59509ab2…`; the
interactive session is kept at `docs/baselines/u8-shell-session.txt`, and the pre- and post-change stdout
captures sit beside it. The full table is `clean-room/specs/010-u8-shell/quickstart.md`, and the pass's own
record is `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §27.

### 188.7 What is owed, and what is not re-opened

Owed: **undock**, the **splitter-drag resize**, the **console's five graded levels**, the **toast's final
form**, and the **P8** `gholtzap/lintapple` gate — which belongs to **U13**
(`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 13), so this slice does not take it and says so in
`clean-room/specs/010-u8-shell/spec.md`. Not re-opened: the four U10 rulings, `mc-ar-mod`, the
`noul`/`score` calibration sets, the score instrument's rungs, U1's and U6's remainders, U11, U12 and U13.
"""
text = read(ledger)
write(ledger, text.rstrip("\r\n") + c188.replace("\n", eol(text)))
print("appended C-188")

# --------------------------------------------------------------- the readme ----
readme = "clean-room/README.md"
patch(readme, [
    ("| [`specs/008-u5-instruments-light-and-audio/spec.md`](specs/008-u5-instruments-light-and-audio/spec.md) | the fifth (U5, instrumentation, light and audio) — the active feature |\n",
     "| [`specs/008-u5-instruments-light-and-audio/spec.md`](specs/008-u5-instruments-light-and-audio/spec.md) | the fifth (U5, instrumentation, light and audio) |\n"
     "| [`specs/009-u7-decision-layer/spec.md`](specs/009-u7-decision-layer/spec.md) | the seventh (U7, the decision layer) |\n"
     "| [`specs/010-u8-shell/spec.md`](specs/010-u8-shell/spec.md) | the eighth (U8, the shell) — the active feature |\n"),
    ("specs/008-u5-instruments-light-and-audio/  the same set, for U5\nspecs/",
     "specs/008-u5-instruments-light-and-audio/  the same set, for U5\nspecs/009-u7-decision-layer/        the same set, for U7\nspecs/010-u8-shell/                 the same set, for U8\nspecs/"),
    ("cargo test --offline        # 323 passed / 0 failed: 263 lib + 2 U4 + 6 U5 + 17 U7 + 7 U10\n                            #            + 5 U6 admit + 7 U6 bridge + 6 U6 intake + 6 U6 transport + 4 U9\n",
     "cargo test --offline        # 334 passed / 0 failed: 274 lib (11 of them U8's) + 2 U4 + 6 U5 + 17 U7 + 7 U10\n                            #            + 5 U6 admit + 7 U6 bridge + 6 U6 intake + 6 U6 transport + 4 U9\n"),
    ("                            # (464 lines of stdout: the U7 slice added five and removed none)\n",
     "                            # (467 lines of stdout: the U8 slice added three and removed none)\n"),
    ("**U8** is `absent`.",
     "**U8** is **built**: the only window — a `Mount` pane tree over a declared `Recipe` (six windows, and the\n"
     "mount adds none it was not handed), the shell/HUD **layer toggle** over one window set, the **one**\n"
     "pointer-ownership decision read once and handed by value to the drawing and the grab, a **four-bucket\n"
     "admitted constants table** (16 rows; an unadmitted name is refused by name), and a visibly drawn\n"
     "**100 × 30** frame whose panes read the registry, the reference world, its materials, the stage roster, a\n"
     "derived chunk's geometry and the light table — with the binary's `shell` mode as the interactive check\n"
     "([`specs/010-u8-shell/quickstart.md`](specs/010-u8-shell/quickstart.md) has the pasted frames). Owed and\n"
     "named: undock, the splitter-drag resize, the console's five graded levels, the toast's final form, and the\n"
     "`gholtzap/lintapple` **P8** gate, which is **U13's**."),
])
print("done")
