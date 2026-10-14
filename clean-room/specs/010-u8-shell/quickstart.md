# U8 quickstart — run it, look at it, and read the numbers back

Every number below was pasted from a run on the final bytes (2026-10-12, §27). `docs/CLEAN-ROOM-PLAN-2026-10-11.md`
§5 row 7's check is *a visibly drawn frame; pointer ownership read once* — the first two commands are how a
person checks it.

## 1. The frame a person looks at

```bash
cd clean-room
cargo run --offline -- shell
```

It draws the frame and then reads one command per line: `split <pane> h|v`, `close <pane>`,
`focus <window>`, `layer`, `grab on|off`, `quit`. A scripted session (`layer`, `split 0 v`,
`focus geometry`, `close 2`, `grab off`, an unknown command, an unknown pane, `quit`) produced **247**
lines: one 30-line frame up front, then a note plus a fresh frame per command — seven commands, seven
notes. The whole session is kept at `docs/baselines/u8-shell-session.txt`.

**The first frame, verbatim.** It is drawn on a 100 × 30 grid, so every line is padded to the 100th
column; the trailing padding is shown **stripped** here (and nowhere else) because it carries no
characters. Nothing else is edited — this is the drawn frame, row for row:

```
shploingus — the shell (slice 7). layer=shell pointer_owner=shell glyph=true grab=true | windows 6 p
layers: shell | hud (2 pinnable) — Escape/F1 | bar 6 cells (48 dp ÷ 8 px) floor 5
pointer: shell (owned by the same read that grants the grab)




┌─ systems · lane=Frame · pinnable=false ──────●─┐┌─ stages · lane=World · pinnable=false ─────────┐
│13 unified systems (docs/CLEAN-ROOM-PLAN-2026-10││declared stages: 15                             │
│  [U1] Presentation — specified                 ││names: tectonics fields geology terrain hydrolog│
│  [U2] World model and its vocabulary — in      ││roster digest: 0e2f704345b47814c2fdbad3183a8290a│
│… 10 more line(s) (the pane is a window, not a r││a stage body that is owed refuses by name, never│
└────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
┌─ world · lane=World · pinnable=true ───────────┐┌─ geometry · lane=World · pinnable=false ───────┐
│identity: seed=1592639215 period=1s tick=0      ││render ladder: 3 levels (max 2, tile up to 4)   │
│truth_digest: 146ff7bf95369b177f3df90ae837d91c95││palette classes: 6                              │
│residency_digest: 98d9c352bbb604ecf12556d93f31f0││chunk (0,0,0): 1289 faces → 313 quads           │
│… 1 more line(s) (the pane is a window, not a re││… 1 more line(s) (the pane is a window, not a re│
└────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
┌─ materials · lane=World · pinnable=false ──────┐┌─ light · lane=Beacon · pinnable=true ──────────┐
│admitted materials: 6                           ││solar constant: 1361.000 W/m²                   │
│ids: 1 2 3 4 5 6                                ││night sky: 0.001 lx                             │
│registry digest: 78ff25de6f9689a1cddf5e70d1f6ff4││channels: 12 over 400–700 nm                    │
│an id the registry has not admitted is refused b││moon: 100 lm/W                                  │
│                                                ││a quantity that cannot be weighed is refused by │
│                                                ││                                                │
│                                                ││                                                │
│                                                ││                                                │
│                                                ││                                                │
│                                                ││                                                │
└────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
```

The focused pane is marked `●` on its title bar (the `systems` pane above), because a text frame cannot
raise one. The bar states its own geometry's derivation (`bar 6 cells (48 dp ÷ 8 px)`), so the admitted
values are checkable rather than asserted.

Then `layer` moves to the HUD. The same frame keeps only the two windows the recipe declares `pinnable` —
the shell and the HUD are **two layers of one window set**, and the pointer goes back to the world because
the shell no longer holds it:

```
shploingus — the shell (slice 7). layer=hud pointer_owner=world glyph=false grab=true | windows 6 pa
layers: shell | hud (2 pinnable) — Escape/F1 | bar 6 cells (48 dp ÷ 8 px) floor 5
…
┌─ world · lane=World · pinnable=true ───────────┐
│identity: seed=1592639215 period=1s tick=0      │
│truth_digest: 146ff7bf95369b177f3df90ae837d91c95│
│residency_digest: 98d9c352bbb604ecf12556d93f31f0│
│… 1 more line(s) (the pane is a window, not a re│
└────────────────────────────────────────────────┘
                                                  ┌─ light · lane=Beacon · pinnable=true ──────────┐
                                                  │solar constant: 1361.000 W/m²                   │
                                                  │night sky: 0.001 lx                             │
                                                  │channels: 12 over 400–700 nm                    │
                                                  │moon: 100 lm/W                                  │
                                                  │a quantity that cannot be weighed is refused by │
                                                  └────────────────────────────────────────────────┘
```

(For brevity the HUD frame's blank rows between the bar and the panes are elided above with `…`; the file
keeps them.)

And the refusals are named where they happened:

```
shell: layer=hud
shell: split pane #0 → #6 (Vertical)
shell: focus geometry → pane #4
shell: closed pane #2
shell: grab=off
shell: refused by name: tool_undeclared: `wibble` is not a shell command (split, close, focus, layer, grab, quit)
shell: refused by name: pane_unknown: pane #999 is not mounted: a pane is addressed by its own identity, never by a list position
```

## 2. The report's own U8 lines

```bash
cargo run --offline
```

Three lines are new, and nothing was removed:

```
U8 shell frame:  100 × 30 cells — 6 windows, 6 panes, layer=shell, pointer_owner=shell, glyph=true
U8 frame digest: 375c5122a80d3cabbf2563d05de8c0065e9e9b92acd801ded11bdfc462f9cd82 (§40; `07adeda4…` was §39's, `79f3885c…` this capture's own, before the mesh moved) (a visibly drawn frame = true, pointer ownership read once = true)
U8 constants:    16 admitted (md1-rule 8, md1-example 2, platform-translation 1, project-extension 5); an unadmitted name is refused by name
```

The stdout line count moved **464 → 467**, and the movement is stated exactly rather than as a bare count:

* **Three lines added** (none removed) — the three U8 lines above, at 464–466.
* **Three lines changed in place** — line 55, the `[U8]` registry line (`absent` → `built (2026-10-12, §27):
  …`); line 60, U8's `interface:` field; and line 90, U13's status sentence, which the fix pass repointed
  (`which no slice owns` → `which its own acceptance check closes`).
* Every other line is **byte-identical**.

Both captures are kept, so the next audit can run the diff rather than trust this table:
`docs/baselines/u8-pre-stdout.txt`
(`sha256 8dbce319a305c90a14420bc2bcd7ffc5b7510ed82c5073d4dfd498dfa24f0efe`) and
`docs/baselines/u8-post-stdout.txt`
(`sha256 b2363fe7acb7311f54570eb674824f8c40d1870b004b13772891e0e0db9362bc`).

## 3. The gates

```bash
cargo test --offline        # 334 passed / 0 failed: 274 library (11 of them U8's) + 60 across nine suites
cargo clippy --offline --all-targets -- -D warnings   # exit 0
cargo run --offline         # exit 0, 467 lines of stdout
```

| what moved | before | after |
|---|---|---|
| `RefusalKind::COUNT` | 77 | **80** (`constant_unadmitted`, `pane_unknown`, `split_floor_breached`) |
| library tests | 263 | **274** (eleven `shell` tests) |
| total tests | 323 | **334** |
| stdout lines | 464 | **467** |
| frames drawn | — | **100 × 30**, digest `375c5122…` (§40; `07adeda4…` was §39's, `79f3885c…` before the mesh moved) |
